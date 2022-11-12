use std::time::Duration;

use crate::entry::{Action, Entry};
use crate::message::{Message, MessageContent};

use super::utils::{assert_no_message, assert_vote, setup_servers, shutdown, Fake};

#[tokio::test]
pub async fn should_accept_vote() {
    let (threads, senders, mut receivers) = setup_servers(1, None, Fake::Server).await;

    let sender = &senders[0];
    let receiver = &mut receivers[0];

    sender
        .send(Message {
            content: MessageContent::VoteRequest {
                last_log_index: 0,
                last_log_term: 0,
            },
            from: 1,
            term: 1,
        })
        .await
        .expect("Send should not fail");

    let message = receiver.recv().await.unwrap();

    assert_eq!(message.content, MessageContent::VoteResponse(true));

    shutdown(senders, threads).await
}

#[tokio::test]
pub async fn should_receive_election() {
    let (threads, senders, mut receivers) =
        setup_servers(1, Some(vec![Duration::from_millis(10)]), Fake::Server).await;

    assert_vote(&mut receivers[0], &senders[0]).await;

    shutdown(senders, threads).await
}

#[tokio::test]
pub async fn should_retry_election() {
    let (threads, senders, mut receivers) =
        setup_servers(1, Some(vec![Duration::from_millis(10)]), Fake::Server).await;

    let sender = &senders[0];
    let receiver = &mut receivers[0];

    let message = receiver.recv().await.unwrap();
    assert_eq!(
        message.content,
        MessageContent::VoteRequest {
            last_log_index: 0,
            last_log_term: 0,
        }
    );

    sender
        .send(Message {
            content: MessageContent::AppendEntries {
                entries: Vec::new(),
                prev_log_index: 0,
                prev_log_term: 0,
                leader_commit: 0,
            },
            term: message.term,
            from: 1,
        })
        .await
        .expect("Send should not fail");

    let message = receiver.recv().await.unwrap();
    assert_eq!(
        message.content,
        MessageContent::AppendResponse {
            success: true,
            match_index: 0
        }
    );

    let message = receiver.recv().await.unwrap();
    assert_eq!(
        message.content,
        MessageContent::VoteRequest {
            last_log_index: 0,
            last_log_term: 0,
        }
    );
    assert_eq!(message.term, 2);

    shutdown(senders, threads).await
}

#[tokio::test]
pub async fn should_elect_first() {
    let (threads, senders, mut receivers) = setup_servers(
        2,
        Some(vec![Duration::from_millis(10), Duration::from_millis(100)]),
        Fake::Server,
    )
    .await;

    let receiver = &mut receivers[0];

    let message = receiver.recv().await.unwrap();
    assert_eq!(
        message.content,
        MessageContent::VoteRequest {
            last_log_index: 0,
            last_log_term: 0,
        }
    );
    assert_eq!(message.from, 0);

    let message = receiver.recv().await.unwrap();
    assert_eq!(
        message.content,
        MessageContent::AppendEntries {
            entries: Vec::new(),
            prev_log_index: 0,
            prev_log_term: 0,
            leader_commit: 0
        }
    );

    assert_eq!(message.term, 1);

    shutdown(senders, threads).await
}

#[tokio::test]
pub async fn setup_server_client_no_recv() {
    let (threads, senders, mut receivers) = setup_servers(
        2,
        Some(vec![Duration::from_millis(10), Duration::from_millis(100)]),
        Fake::Client,
    )
    .await;

    let client_receiver = &mut receivers[0];

    assert_no_message(client_receiver).await;
    shutdown(senders, threads).await
}

#[tokio::test]
pub async fn client_send_request() {
    let (threads, senders, mut receivers) = setup_servers(
        2,
        Some(vec![Duration::from_millis(100), Duration::from_millis(500)]),
        Fake::Client,
    )
    .await;

    let client_receiver = &mut receivers[0];

    tokio::time::sleep(Duration::from_millis(125)).await;

    let leader = &senders[0];
    leader
        .send(Message {
            content: MessageContent::ClientRequest(Action::Set {
                key: String::from("key"),
                value: String::from("value"),
            }),
            term: 0,
            from: 2, // client
        })
        .await
        .unwrap();

    let resp = client_receiver.recv().await.unwrap();
    assert_eq!(
        resp.content,
        MessageContent::ClientResponse(Ok(String::from("key")))
    );

    assert_no_message(client_receiver).await;
    shutdown(senders, threads).await
}

#[tokio::test]
pub async fn client_server_should_receive_entry() {
    let (threads, senders, mut receivers) = setup_servers(
        2,
        Some(vec![Duration::from_millis(100), Duration::from_millis(500)]),
        Fake::ClientServer,
    )
    .await;

    let mut server_receiver = receivers.pop_front().unwrap();
    let mut client_receiver = receivers.pop_front().unwrap();

    assert_vote(&mut server_receiver, &senders[0]).await;

    let leader = &senders[0];
    leader
        .send(Message {
            content: MessageContent::ClientRequest(Action::Set {
                key: String::from("key"),
                value: String::from("value"),
            }),
            term: 0,
            from: 3, // client
        })
        .await
        .unwrap();

    let resp = client_receiver.recv().await.unwrap();
    assert_eq!(
        resp.content,
        MessageContent::ClientResponse(Ok(String::from("key")))
    );

    let resp = server_receiver.recv().await.unwrap();
    let expected_content = MessageContent::AppendEntries {
        entries: vec![Entry {
            term: 1,
            action: Action::Set {
                key: String::from("key"),
                value: String::from("value"),
            },
        }],
        prev_log_index: 0,
        prev_log_term: 0,
        leader_commit: 0,
    };
    assert_eq!(resp.content, expected_content);

    leader.send(Message {
        content: MessageContent::AppendResponse {
            success: true,
            match_index: 1,
        },
        term: 1,
        from: 2,
    }).await.unwrap();

    let resp = server_receiver.recv().await.unwrap();
    let expected_content = MessageContent::AppendEntries {
        entries: vec![],
        prev_log_index: 1,
        prev_log_term: 1,
        leader_commit: 1,
    };
    assert_eq!(resp.content, expected_content);

    assert_no_message(&mut client_receiver).await;
    shutdown(senders, threads).await
}
