use std::{pin::Pin, time::Duration};

use tokio::{
    sync::mpsc::{Receiver, Sender},
    time::Sleep,
};

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
    shutdown_requested: bool,
    simulate_crash: bool,
    pub(crate) election_timeout_range: (Duration, Duration),

    // Persistent state (for all server)
    role: Role,
    current_term: usize,
    voted_for: NodeId,
    logs: Vec<Entry>,

    // Volatile state (for all servers)
    candidate_votes: usize,
    commit_index: usize,
    last_applied: usize,

    // Volatile state (for leaders, reinitialized after election)
    next_index: Vec<usize>,
    match_index: Vec<usize>,
}

impl Node {
    pub fn new(id: NodeId, receiver: Receiver<Message>, senders: Vec<Sender<Message>>) -> Self {
        let node_count = senders.len();

        Self {
            id,
            receiver,
            senders,
            shutdown_requested: false,
            simulate_crash: false,
            election_timeout_range: CONFIG.election_timeout_range(),

            role: Role::Follower,
            current_term: 0,
            // Will never be used at init
            voted_for: id,
            logs: Vec::new(),

            candidate_votes: 0,
            commit_index: 0,
            last_applied: 0,

            next_index: vec![1; node_count],
            match_index: vec![0; node_count],
        }
    }

    pub async fn run(&mut self) {
        let mut timeout = self.generate_timeout();

        loop {
            if self.shutdown_requested {
                break;
            }

            tokio::select! {
                msg = self.receiver.recv() => {
                    println!("Server {} received {:?}", self.id,msg);
                    match msg {
                        Some(Message { content: MessageContent::Repl(action), ..}) => {
                            self.handle_repl(action)
                        },
                        Some(msg) => {
                            if !self.simulate_crash {
                                self.handle_message(msg).await;

                                // Refresh timeout
                                timeout = self.generate_timeout();
                            }
                        }
                        None => {
                            // We have been disconnected
                        }
                    }
                },
                _ = &mut timeout => {
                    // Refresh timeout
                    timeout = self.generate_timeout();

                    if self.simulate_crash {
                        continue;
                    }

                    if self.role != Role::Leader {
                        println!("Server {} received error, Timeout", self.id);
                        self.start_election().await;
                    } else {
                        println!("Server {} sending heartbeat", self.id);
                        self.send_heartbeat().await;
                    }
                }
            }
        }
    }

    fn generate_timeout(&self) -> Pin<Box<Sleep>> {
        let duration = {
            let (min, max) = self.election_timeout_range;

            if self.role == Role::Leader {
                min / 2
            } else if max == min {
                min
            } else {
                let timeout =
                    rand::random::<u128>() % (max.as_millis() - min.as_millis()) + min.as_millis();
                Duration::from_millis(timeout as u64)
            }
        };

        Box::pin(tokio::time::sleep(duration))
    }

    async fn handle_message(&mut self, message: Message) {
        let Message {
            term,
            from,
            content,
        } = message;

        match content {
            MessageContent::VoteRequest {
                last_log_index,
                last_log_term,
            } => {
                let accept = term > self.current_term
                    && last_log_term >= self.logs.last().map(|e| e.term).unwrap_or(0)
                    && last_log_index >= self.logs.len();

                if accept {
                    // TODO: check if up to date
                    self.current_term = term;
                    self.voted_for = from;
                    self.role = Role::Follower;
                }

                self.emit(from, MessageContent::VoteResponse(accept)).await;
            }
            MessageContent::VoteResponse(granted) => {
                if self.current_term == term && self.role == Role::Candidate && granted {
                    self.candidate_votes += 1;
                    if self.candidate_votes > self.senders.len() / 2 {
                        self.promote_leader().await;
                    }
                }
            }
            MessageContent::Repl(action) => match action {
                ReplAction::Shutdown => {
                    self.shutdown_requested = true;
                }
                _ => {}
            },
            MessageContent::AppendEntries { logs } => {
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
        self.broadcast(MessageContent::VoteRequest {
            last_log_index: self.logs.len(),
            last_log_term: self.logs.last().map(|e| e.term).unwrap_or(0),
        })
        .await;
    }

    async fn promote_leader(&mut self) {
        self.role = Role::Leader;

        for i in 0..self.senders.len() {
            self.next_index[i] = self.logs.len();
            self.match_index[i] = 0;
        }

        self.send_heartbeat().await;
    }

    async fn send_heartbeat(&mut self) {
        assert!(self.role == Role::Leader);

        self.broadcast(MessageContent::AppendEntries { logs: vec![] })
            .await;
    }

    fn handle_repl(&mut self, action: ReplAction) {
        match action {
            ReplAction::Crash => {
                self.simulate_crash = true;
            }
            ReplAction::Start => {
                self.simulate_crash = false;
            }
            ReplAction::Shutdown => {
                self.shutdown_requested = true;
            }
            _ => {}
        }
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
        let message = Message {
            content,
            term: self.current_term,
            from: self.id,
        };

        for (i, sender) in self.senders.iter().enumerate() {
            if i == self.id {
                continue;
            }

            let res = sender.send(message.clone()).await;
            if let Err(err) = res {
                dbg!(err);
            }
        }
    }
}
