use std::time::Duration;

use tokio::sync::mpsc::{Receiver, Sender};

use crate::{
    entry::Entry,
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
    logs: Vec<Entry>,

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
            logs: Vec::new(),

            candidate_votes: 0,
            shutdown_requested: false,
            election_timeout_range: CONFIG.election_timeout_range(),
        }
    }

    pub async fn run(&mut self) {
        loop {
            if self.shutdown_requested {
                break;
            }

            let timeout = self.generate_timeout();

            tokio::select! {
                msg = self.receiver.recv() => {
                    println!("Server {} received {:?}", self.id,msg);
                    if let Some(msg) = msg {
                        self.handle_message(msg).await;
                    }
                }
                _ = tokio::time::sleep(timeout) => {
                    if self.role != Role::Leader {
                        println!("Server {} received error, Timeout", self.id);
                        self.start_election().await;
                    } else {
                        println!("Server {} sending heartbeat", self.id);
                        self.send_heartbeat().await;
                    }
                }
                // Missing Disconnect check
            }
        }
    }

    fn generate_timeout(&self) -> Duration {
        let (min, max) = self.election_timeout_range;

        if self.role == Role::Leader {
            return min / 2;
        }

        if max == min {
            return min;
        }

        let timeout =
            rand::random::<u128>() % (max.as_millis() - min.as_millis()) + min.as_millis();
        Duration::from_millis(timeout as u64)
    }

    async fn handle_message(&mut self, message: Message) {
        let Message {
            term,
            from,
            content,
        } = message;

        match content {
            MessageContent::VoteRequest => {
                let accept = term > self.current_term;
                if accept {
                    // TODO: check if up to date
                    self.current_term = term;
                    self.voted_for = from;
                    self.role = Role::Follower;
                }

                self.emit(from, MessageContent::VoteResponse(accept)).await;
            }
            MessageContent::VoteResponse(granted) => {
                // Maybe no need to check for Role::Candidate
                if self.current_term == term && self.role == Role::Candidate && granted {
                    self.candidate_votes += 1;
                    if self.candidate_votes > self.senders.len() / 2 {
                        self.role = Role::Leader;
                        self.send_heartbeat().await;
                    }
                }
            }
            MessageContent::Repl(action) => match action {
                ReplAction::Shutdown => {
                    self.shutdown_requested = true;
                }
                _ => {}
            },
            MessageContent::AppendEntries { leader_id, logs } => {
                let accept = term >= self.current_term;
                if term >= self.current_term {
                    self.current_term = term;
                    // TODO: fix should verify / override on conditions
                    self.logs.extend(logs);

                    self.role = Role::Follower;
                }

                self.emit(from, MessageContent::AppendResponse(accept))
                    .await;
            }
            _ => (),
        }
    }

    async fn start_election(&mut self) {
        self.role = Role::Candidate;
        self.current_term += 1;
        self.candidate_votes = 1;
        self.voted_for = self.id;
        self.broadcast(MessageContent::VoteRequest).await;
    }

    async fn send_heartbeat(&mut self) {
        assert!(self.role == Role::Leader);

        self.broadcast(MessageContent::AppendEntries {
            logs: vec![],
            leader_id: self.id,
        })
        .await;
    }

    async fn emit(&self, id: NodeId, content: MessageContent) {
        let res = self.senders[id]
            .send(Message {
                content,
                term: self.current_term,
                from: self.id,
            })
            .await;

        if let Err(err) = res {
            dbg!(err);
        }
    }

    async fn broadcast(&self, content: MessageContent) {
        for (i, sender) in self.senders.iter().enumerate() {
            if i == self.id {
                continue;
            }

            let res = sender
                .send(Message {
                    content: content.clone(),
                    term: self.current_term,
                    from: self.id,
                })
                .await;
            if let Err(err) = res {
                dbg!(err);
            }
        }
    }
}
