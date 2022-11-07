use std::{
    sync::mpsc::{Receiver, RecvTimeoutError, Sender},
    time::Duration,
};

use crate::{
    message::{Message, MessageContent, ReplAction},
    CONFIG,
};

pub type NodeId = usize;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Role {
    Follower,
    Candidate,
    Leader,
}

pub struct Node {
    id: NodeId,
    receiver: Receiver<Message>,
    senders: Vec<Sender<Message>>,

    // Persistent state (for all server)
    role: Role,
    current_term: usize,
    voted_for: NodeId,

    // Volatile state (for all servers)
    candidate_votes: usize,
    shutdown_requested: bool,
    pub(crate) election_timeout_range: (Duration, Duration),
}

impl Node {
    pub fn new(id: NodeId, receiver: Receiver<Message>, senders: Vec<Sender<Message>>) -> Self {
        Self {
            id,
            receiver,
            senders,
            role: Role::Follower,
            current_term: 0,
            // Will never be used at init
            voted_for: id,
            candidate_votes: 0,
            shutdown_requested: false,
            election_timeout_range: CONFIG.election_timeout_range(),
        }
    }

    pub fn run(&mut self) {
        loop {
            if self.shutdown_requested {
                break;
            }

            let timeout = self.generate_timeout();

            match self.receiver.recv_timeout(timeout) {
                Ok(message) => {
                    println!("Server {} received {:?}", self.id, message);
                    // We receive something (server is not dead)
                    self.handle_message(message)
                }
                Err(RecvTimeoutError::Timeout) => {
                    // Relaunch election
                    if self.role != Role::Leader {
                        println!("Server {} received error, Timeout", self.id);
                        self.start_election();
                    } else {
                        println!("Server {} sending heartbeat", self.id);
                        self.broadcast(MessageContent::Heartbeat);
                    }
                }
                Err(RecvTimeoutError::Disconnected) => {
                    println!("Server {} received error, Disconnected", self.id);
                    // Don't know (yet?)
                }
            }
        }
    }

    fn generate_timeout(&self) -> Duration {
        let (min, max) = self.election_timeout_range;

        if self.role == Role::Leader {
            return min / 2;
        }

        if max == min {
            // This should never happen (bad config)
            return min;
        }

        let timeout =
            rand::random::<u128>() % (max.as_millis() - min.as_millis()) + min.as_millis();
        Duration::from_millis(timeout as u64)
    }

    fn handle_message(&mut self, message: Message) {
        let Message {
            term,
            from,
            content,
        } = message;

        match content {
            MessageContent::VoteRequest => {
                if term <= self.current_term {
                    // Reply false
                    self.emit(from, MessageContent::VoteResponse(false));
                } else {
                    // TODO: check if up to date
                    self.current_term = term;
                    self.voted_for = from;
                    self.role = Role::Follower;
                    self.emit(from, MessageContent::VoteResponse(true));
                }
            }
            MessageContent::VoteResponse(granted) => {
                // Maybe no need to check for Role::Candidate
                if self.current_term == term && self.role == Role::Candidate && granted {
                    self.candidate_votes += 1;
                    if self.candidate_votes > self.senders.len() / 2 {
                        self.role = Role::Leader;
                        self.broadcast(MessageContent::Heartbeat);
                    }
                }
            }
            MessageContent::Heartbeat => {
                let accept = term >= self.current_term;
                if accept {
                    self.current_term = term;
                    self.role = Role::Follower;
                }
                self.emit(from, MessageContent::AppendResponse(accept))
            }
            MessageContent::Repl(action) => match action {
                ReplAction::Shutdown => {
                    self.shutdown_requested = true;
                }
                // ReplAction::SetTimeout(timeout) => {
                //     self.election_timeout_range = (timeout, timeout);
                // }
                _ => {}
            },
            _ => (),
        }
    }

    fn start_election(&mut self) {
        self.role = Role::Candidate;
        self.current_term += 1;
        self.candidate_votes = 1;
        self.voted_for = self.id;
        self.broadcast(MessageContent::VoteRequest);
    }

    fn emit(&self, id: NodeId, content: MessageContent) {
        let res = self.senders[id].send(Message {
            content,
            term: self.current_term,
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
                term: self.current_term,
                from: self.id,
            });
            if let Err(err) = res {
                dbg!(err);
            }
        }
    }
}
