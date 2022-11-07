use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::{self, JoinHandle};

use std::collections::VecDeque;
use std::time::Duration;

use crate::message::{Message, MessageContent, ReplAction::*};
use crate::node::Node;

fn setup_serv(
    count: usize,
    timeouts: Option<Vec<Duration>>,
) -> (Vec<JoinHandle<()>>, Vec<Sender<Message>>, Receiver<Message>) {
    assert!(timeouts
        .clone()
        .map_or_else(|| true, |durations| durations.len() == count));

    let mut threads = Vec::with_capacity(count);
    let mut senders = Vec::with_capacity(count + 1);
    let mut receivers = VecDeque::with_capacity(count + 1);

    for _ in 0..count + 1 {
        let (sender, receiver) = mpsc::channel::<Message>();

        senders.push(sender);
        receivers.push_back(receiver);
    }

    for id in 0..count {
        // The sender endpoint can be copied
        let receiver = receivers.pop_front().unwrap();
        let senders = senders.clone();

        let timeouts = timeouts.clone();

        let child = thread::spawn(move || {
            let mut node = Node::new(id, receiver, senders);
            if let Some(durations) = timeouts {
                node.election_timeout_range = (durations[id], durations[id]);
            }
            node.run();
        });

        threads.push(child);
    }

    return (threads, senders, receivers.pop_front().unwrap());
}

fn shutdown(senders: Vec<Sender<Message>>, threads: Vec<JoinHandle<()>>) {
    for sender in senders {
        let _ = sender
            .send(Message {
                content: MessageContent::Repl(Shutdown),
                term: usize::MAX,
                from: usize::MAX,
            })
            .unwrap();
    }

    for thread in threads {
        thread.join().unwrap();
    }
}

#[test]
pub fn should_accept_vote() {
    let (threads, senders, receiver) = setup_serv(1, None);

    let sender = &senders[0];

    sender
        .send(Message {
            content: MessageContent::VoteRequest,
            term: 1,
            from: 1,
        })
        .expect("Send should not fail");

    let message = receiver.recv().unwrap();

    assert_eq!(message.content, MessageContent::VoteResponse(true));

    shutdown(senders, threads)
}

#[test]
pub fn should_receive_election() {
    let (threads, senders, receiver) = setup_serv(1, Some(vec![Duration::from_millis(10)]));

    let sender = &senders[0];

    let message = receiver.recv().unwrap();
    assert_eq!(message.content, MessageContent::VoteRequest);

    sender
        .send(Message {
            content: MessageContent::VoteResponse(true),
            term: message.term,
            from: 1,
        })
        .expect("Send should not fail");

    let message = receiver.recv().unwrap();
    assert_eq!(
        message.content,
        MessageContent::AppendEntries {
            logs: Vec::new(),
            leader_id: 0
        }
    );

    shutdown(senders, threads)
}

#[test]
pub fn should_retry_election() {
    let (threads, senders, receiver) = setup_serv(1, Some(vec![Duration::from_millis(10)]));

    let sender = &senders[0];

    let message = receiver.recv().unwrap();
    assert_eq!(message.content, MessageContent::VoteRequest);

    sender
        .send(Message {
            content: MessageContent::AppendEntries {
                logs: Vec::new(),
                leader_id: 1,
            },
            term: message.term,
            from: 1,
        })
        .expect("Send should not fail");

    let message = receiver.recv().unwrap();
    assert_eq!(message.content, MessageContent::AppendResponse(true));

    let message = receiver.recv().unwrap();
    assert_eq!(message.content, MessageContent::VoteRequest);
    assert_eq!(message.term, 2);

    shutdown(senders, threads)
}

#[test]
pub fn should_elect_first() {
    let (threads, senders, receiver) = setup_serv(
        2,
        Some(vec![Duration::from_millis(10), Duration::from_millis(100)]),
    );

    let message = receiver.recv().unwrap();
    assert_eq!(message.content, MessageContent::VoteRequest);

    let message = receiver.recv().unwrap();
    assert_eq!(
        message.content,
        MessageContent::AppendEntries {
            logs: Vec::new(),
            leader_id: 0
        }
    );

    assert_eq!(message.term, 1);

    shutdown(senders, threads)
}
