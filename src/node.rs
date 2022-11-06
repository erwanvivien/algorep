use std::{
    sync::mpsc::{Receiver, RecvTimeoutError, Sender},
    time::Duration,
};

use crate::{
    message::{Message, MessageContent},
    CONFIG,
};

pub type NodeId = usize;

fn generate_timeout() -> Duration {
    let (min, max) = CONFIG.election_timeout;
    let (min, max): (Duration, Duration) = (min.into(), max.into());

    let timeout = rand::random::<u128>() % (max.as_millis() - min.as_millis()) + min.as_millis();
    Duration::from_millis(timeout as u64)
}

pub struct Node {
    id: NodeId,
    receiver: Receiver<Message>,
    senders: Vec<Sender<Message>>,
    timeout: Duration,
}

impl Node {
    pub fn new(id: NodeId, receiver: Receiver<Message>, senders: Vec<Sender<Message>>) -> Self {
        Self {
            id,
            receiver,
            senders,
            timeout: generate_timeout(),
        }
    }

    pub fn run(&self) {
        self.broadcast(MessageContent::Vote(self.id));

        loop {
            match self.receiver.recv_timeout(self.timeout) {
                Ok(message) => {
                    println!("Server {} received {:?}", self.id, message);
                    // We receive something (server is not dead)
                    continue;
                }
                Err(RecvTimeoutError::Timeout) => {
                    println!("Server {} received error, Timeout", self.id);

                    // Relaunch election
                }
                Err(RecvTimeoutError::Disconnected) => {
                    println!("Server {} received error, Disconnected", self.id);

                    // Don't know (yet?)
                }
            }
        }
    }

    fn request_vote(&self) {
        self.broadcast(MessageContent::RequestVote);
    }

    fn emit(&self, content: MessageContent) {
        let res = self.senders[self.id].send(Message {
            content,
            from: self.id,
        });

        if let Err(err) = res {
            dbg!(err);
        }
    }

    fn broadcast(&self, content: MessageContent) {
        for (i, sender) in self.senders.iter().enumerate() {
            if i == self.id {
                continue;
            }

            let res = sender.send(Message {
                content: content.clone(),
                from: self.id,
            });
            if let Err(err) = res {
                dbg!(err);
            }
        }
    }
}
