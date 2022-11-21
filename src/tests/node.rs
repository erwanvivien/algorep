use std::cmp::min;
use std::time::Duration;

use crate::entry::{LogEntry, StateMutation};
use crate::message::{ClientCommand, ClientResponse, Message, MessageContent};

use super::utils::{assert_no_message, assert_vote, recv_timeout, setup_servers, shutdown, Fake};

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
                term: 1,
            },
            from: 1,
        })
        .await
        .expect("Send should not fail");

    let message = receiver.recv().await.unwrap();

    assert_eq!(
        message.content,
        MessageContent::VoteResponse {
            granted: true,
            term: 1
        }
    );

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
            term: 1
        }
    );

    sender
        .send(Message {
            content: MessageContent::AppendEntries {
                entries: Vec::new(),
                prev_log_index: 0,
                prev_log_term: 0,
                leader_commit: 0,
                term: 1,
            },
            from: 1,
        })
        .await
        .expect("Send should not fail");

    let message = receiver.recv().await.unwrap();
    assert_eq!(
        message.content,
        MessageContent::AppendResponse {
            success: true,
            match_index: 0,
            term: 1
        }
    );

    let message = receiver.recv().await.unwrap();
    assert_eq!(
        message.content,
        MessageContent::VoteRequest {
            last_log_index: 0,
            last_log_term: 0,
            term: 2
        }
    );

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
            term: 1
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
            leader_commit: 0,
            term: 1
        }
    );

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

    tokio::time::sleep(Duration::from_millis(120)).await;

    let leader = &senders[0];
    leader
        .send(Message {
            content: MessageContent::ClientRequest(ClientCommand::Load {
                filename: "my_file".to_string(),
            }),
            from: 2, // client
        })
        .await
        .unwrap();

    let resp = client_receiver.recv().await.unwrap();
    assert_eq!(
        resp.content,
        MessageContent::ClientResponse(Ok(ClientResponse::Uid("1-1".to_string())))
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
            content: MessageContent::ClientRequest(ClientCommand::Load {
                filename: "my_file".to_string(),
            }),
            from: 3, // client
        })
        .await
        .unwrap();

    let resp = client_receiver.recv().await.unwrap();
    assert_eq!(
        resp.content,
        MessageContent::ClientResponse(Ok(ClientResponse::Uid("1-1".to_string())))
    );

    let resp = server_receiver.recv().await.unwrap();
    let expected_content = MessageContent::AppendEntries {
        entries: vec![LogEntry {
            term: 1,
            mutation: StateMutation::Create {
                uid: "1-1".to_string(),
                filename: "my_file".to_string(),
            },
        }],
        prev_log_index: 0,
        prev_log_term: 0,
        leader_commit: 0,
        term: 1,
    };
    assert_eq!(resp.content, expected_content);

    leader
        .send(Message {
            content: MessageContent::AppendResponse {
                success: true,
                match_index: 1,
                term: 1,
            },
            from: 2,
        })
        .await
        .unwrap();

    let resp = server_receiver.recv().await.unwrap();
    let expected_content = MessageContent::AppendEntries {
        entries: vec![],
        prev_log_index: 1,
        prev_log_term: 1,
        leader_commit: 1,
        term: 1,
    };
    assert_eq!(resp.content, expected_content);

    assert_no_message(&mut client_receiver).await;
    shutdown(senders, threads).await
}

#[tokio::test]
pub async fn should_timeout() {
    let (threads, senders, mut receivers) =
        setup_servers(1, Some(vec![Duration::from_millis(100)]), Fake::Server).await;

    let mut server_receiver = receivers.pop_front().unwrap();
    let leader = &senders[0];

    leader
        .send(Message {
            content: MessageContent::VoteRequest {
                last_log_index: 0,
                last_log_term: 0,
                term: 5,
            },
            from: 1,
        })
        .await
        .unwrap();

    let resp = server_receiver.recv().await.unwrap();
    assert_eq!(
        resp.content,
        MessageContent::VoteResponse {
            term: 5,
            granted: true
        }
    );

    tokio::time::sleep(Duration::from_millis(50)).await;

    leader
        .send(Message {
            content: MessageContent::AppendEntries {
                entries: vec![],
                prev_log_index: 0,
                prev_log_term: 0,
                leader_commit: 0,
                term: 0,
            },
            from: 1,
        })
        .await
        .unwrap();

    let resp = server_receiver.recv().await.unwrap();
    assert_eq!(
        resp.content,
        MessageContent::AppendResponse {
            success: false,
            match_index: 0,
            term: 5
        }
    );

    assert!(
        recv_timeout(&mut server_receiver, Duration::from_millis(75))
            .await
            .is_some()
    );
    shutdown(senders, threads).await
}

#[tokio::test]
pub async fn should_handle_append_entries() {
    let (threads, senders, mut receivers) =
        setup_servers(1, Some(vec![Duration::from_millis(50)]), Fake::ClientServer).await;

    let mut server_receiver = receivers.pop_front().unwrap();
    let server_0 = &senders[0];
    let mut client_receiver = receivers.pop_front().unwrap();

    let mut previous_count = 0;

    for i in 1..10 {
        let entries = (1..=i)
            .map(|j| LogEntry {
                term: 1,
                mutation: StateMutation::Create {
                    uid: format!("uid {}", i * 10 + j),
                    filename: format!("filename {}", i * 10 + j),
                },
            })
            .collect::<Vec<_>>();

        let msg = Message {
            content: MessageContent::AppendEntries {
                entries,
                prev_log_index: previous_count,
                prev_log_term: min(previous_count, 1),
                leader_commit: previous_count,
                term: 1,
            },
            from: 1,
        };

        server_0.send(msg).await.unwrap();
        previous_count += i;

        let expected_content = MessageContent::AppendResponse {
            success: true,
            match_index: previous_count,
            term: 1,
        };

        let resp = server_receiver.recv().await.unwrap();
        assert_eq!(resp.content, expected_content);
    }

    let msg = Message {
        content: MessageContent::AppendEntries {
            entries: vec![],
            prev_log_index: previous_count,
            prev_log_term: 1,
            leader_commit: previous_count,
            term: 1,
        },
        from: 1,
    };

    // Test that match_index do not decrement
    for _ in 1..10 {
        let expected_content = MessageContent::AppendResponse {
            success: true,
            match_index: previous_count,
            term: 1,
        };

        server_0.send(msg.clone()).await.unwrap();
        let resp = server_receiver.recv().await.unwrap();
        assert_eq!(resp.content, expected_content);
    }

    // Let the server timeout
    let message = server_receiver.recv().await.unwrap();
    assert_eq!(
        message.content,
        MessageContent::VoteRequest {
            last_log_index: previous_count,
            last_log_term: 1,
            term: 2
        }
    );

    server_0
        .send(Message {
            content: MessageContent::VoteResponse {
                granted: true,
                term: 2,
            },
            from: 1,
        })
        .await
        .expect("Send should not fail");

    let message = server_receiver.recv().await.unwrap();
    assert_eq!(
        message.content,
        MessageContent::AppendEntries {
            entries: Vec::new(),
            prev_log_index: previous_count,
            prev_log_term: 1,
            leader_commit: previous_count,
            term: 2
        }
    );

    for i in 1..10 {
        for j in 1..=i {
            let uid = i * 10 + j;

            // Send a get request to server
            server_0
                .send(Message {
                    content: MessageContent::ClientRequest(ClientCommand::Get {
                        uid: format!("uid {}", uid),
                    }),
                    from: 2, // Client
                })
                .await
                .unwrap();

            let resp = client_receiver.recv().await.unwrap();
            assert_eq!(
                resp.content,
                MessageContent::ClientResponse(Ok(ClientResponse::File(
                    crate::node::volatile_state::File {
                        filename: format!("filename {}", uid),
                        text: "".to_string(),
                    }
                )))
            );
        }
    }

    shutdown(senders, threads).await
}
