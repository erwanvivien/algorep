use std::sync::mpsc::{Receiver, Sender};

use crate::message::{Message, MessageContent};

pub type NodeId = usize;

pub struct Node {
    id: NodeId,
    receiver: Receiver<Message>,
    senders: Vec<Sender<Message>>,
}

impl Node {
    pub const fn new(
        id: NodeId,
        receiver: Receiver<Message>,
        senders: Vec<Sender<Message>>,
    ) -> Self {
        Self {
            id,
            receiver,
            senders,
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

    pub fn run(&self) {
        self.broadcast(MessageContent::Vote(self.id));

        loop {
            match self.receiver.recv() {
                Ok(message) => {
                    println!("Server {} received {:?}", self.id, message);
                }
                Err(err) => {
                    println!("Server {} received error, {:?}", self.id, err);
                }
            }
        }
    }
}
