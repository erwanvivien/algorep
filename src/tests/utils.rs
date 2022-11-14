use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio::task::JoinHandle;

use std::collections::VecDeque;
use std::time::Duration;

use crate::message::{Message, MessageContent, ReplAction::*};
use crate::node::Node;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Fake {
    Client = 0,
    Server = 1,
    ClientServer = 2,
}

pub async fn setup_servers(
    count: usize,
    timeouts: Option<Vec<Duration>>,
    fake: Fake,
) -> (
    Vec<JoinHandle<()>>,
    Vec<Sender<Message>>,
    VecDeque<Receiver<Message>>,
) {
    assert!(timeouts
        .clone()
        .map_or_else(|| true, |durations| durations.len() == count));

    let mut threads = Vec::with_capacity(count);

    let channels_count = match fake {
        Fake::Client => count + 1,
        Fake::Server => count + 1,
        Fake::ClientServer => count + 2,
    };
    let mut senders = Vec::with_capacity(channels_count);
    let mut receivers = VecDeque::with_capacity(channels_count);

    for _ in 0..channels_count {
        let (sender, receiver) = mpsc::channel::<Message>(4096);

        senders.push(sender);
        receivers.push_back(receiver);
    }

    for id in 0..count {
        // The sender endpoint can be copied
        let receiver = receivers.pop_front().unwrap();
        let senders = senders.clone();

        let timeouts = timeouts.clone();

        let child = tokio::spawn(async move {
            let node_count = match fake {
                Fake::Client => count,
                Fake::Server => count + 1,
                Fake::ClientServer => count + 1,
            };
            let mut node = Node::new(id, node_count, receiver, senders);
            if let Some(durations) = timeouts {
                node.election_timeout_range = (durations[id], durations[id]);
            }
            node.run().await;
        });

        threads.push(child);
    }

    return (threads, senders, receivers);
}

pub async fn shutdown(senders: Vec<Sender<Message>>, threads: Vec<JoinHandle<()>>) {
    for sender in senders {
        let _ = sender
            .send(Message {
                content: MessageContent::Repl(Shutdown),
                term: usize::MAX,
                from: usize::MAX,
            })
            .await
            .unwrap();
    }

    for thread in threads {
        thread.await.unwrap();
    }
}

pub async fn assert_vote(fake_receiver: &mut Receiver<Message>, fake_sender: &Sender<Message>) {
    let message = fake_receiver.recv().await.unwrap();
    assert_eq!(
        message.content,
        MessageContent::VoteRequest {
            last_log_index: 0,
            last_log_term: 0,
        }
    );

    fake_sender
        .send(Message {
            content: MessageContent::VoteResponse(true),
            term: message.term,
            from: 1,
        })
        .await
        .expect("Send should not fail");

    let message = fake_receiver.recv().await.unwrap();
    assert_eq!(
        message.content,
        MessageContent::AppendEntries {
            entries: Vec::new(),
            prev_log_index: 0,
            prev_log_term: 0,
            leader_commit: 0
        }
    );
}

pub async fn assert_no_message(receiver: &mut Receiver<Message>) {
    assert_eq!(
        recv_timeout(receiver, Duration::from_millis(10)).await,
        None
    );
}

pub async fn recv_timeout(receiver: &mut Receiver<Message>, dur: Duration) -> Option<Message> {
    let tmp = tokio::select! {
        message = receiver.recv() => {
            message
        }
        _ = tokio::time::sleep(dur) => {
            None
        }
    };

    return tmp;
}
