mod client;
mod election;
mod networking;
mod repl;
mod role;
mod server;
mod utils;

pub(crate) mod volatile_state;

use std::{collections::VecDeque, time::Duration};

use log::{debug, info};

use tokio::sync::mpsc::{Receiver, Sender};

use crate::{
    entry::{LogEntry, StateMutation},
    message::{ClientResponseError, Message, MessageContent, Speed},
    CONFIG,
};

use role::Role;
use volatile_state::VolatileState;

pub type NodeId = usize;
pub type ClientId = usize;

pub struct Node {
    id: NodeId,
    node_count: usize,
    receiver: Receiver<Message>,
    senders: Vec<Sender<Message>>,
    shutdown_requested: bool,
    simulate_crash: bool,
    pub(crate) election_timeout_range: (Duration, Duration),
    leader_id: Option<NodeId>,
    speed: Speed,

    role: Role,

    // Persistent state (for all server)
    current_term: usize,
    voted_for: Option<NodeId>,
    logs: Vec<LogEntry>,

    // Volatile state (for all servers)
    state: VolatileState,
}

impl Node {
    pub fn new(
        id: NodeId,
        node_count: usize,
        receiver: Receiver<Message>,
        senders: Vec<Sender<Message>>,
    ) -> Self {
        Self {
            id,
            node_count,
            receiver,
            senders,
            shutdown_requested: false,
            simulate_crash: false,
            election_timeout_range: CONFIG.election_timeout_range(),
            leader_id: None,
            speed: Speed::Fast,

            role: Role::Follower,
            current_term: 0,
            voted_for: None,
            logs: Vec::new(),

            state: VolatileState::new(),
        }
    }

    pub async fn run(&mut self) {
        let mut timeout = self.generate_timeout();

        loop {
            if self.shutdown_requested {
                break;
            }

            tokio::time::sleep(self.speed.into()).await;

            tokio::select! {
                // Biased means recv is prioritized over timeout
                biased;
                msg = self.receiver.recv() => {
                    debug!("Server {} received {:?}", self.id, msg);
                    if msg.is_none() {
                        continue;
                    }

                    let msg = msg.unwrap();
                    match msg {
                        Message { content: MessageContent::Repl(action), ..} => {
                            self.handle_repl(action).await
                        },
                        Message { content: MessageContent::ClientRequest(_), .. } => {
                            self.handle_client_command(msg).await
                        },
                        msg => {
                            if !self.simulate_crash {
                                let should_reset_timeout = self.handle_message(msg).await
                                    && !self.role.is_candidate();
                                if should_reset_timeout {
                                    timeout = self.generate_timeout();
                                };
                            }
                        }
                    }
                },
                _ = &mut timeout => {
                    // Refresh timeout
                    timeout = self.generate_timeout();

                    if self.simulate_crash {
                        continue;
                    }

                    if !self.role.is_leader() {
                        debug!("Server {} received error, Timeout", self.id);
                        self.start_election().await;
                    } else {
                        debug!("Server {} sending heartbeat", self.id);
                        self.send_entries().await;
                    }
                }
            }

            self.state.apply_committed_entries(&self.logs);
            self.notify_waiters().await;
        }
    }
}

impl Node {
    async fn send_entries(&self) {
        if let Role::Leader(leader) = &self.role {
            for follower in 0..self.node_count {
                if follower == self.id {
                    continue;
                }

                let next_index = leader.next_index[follower];
                self.emit(
                    follower,
                    MessageContent::AppendEntries {
                        prev_log_index: next_index - 1,
                        prev_log_term: if next_index > 1 {
                            self.logs[next_index - 2].term
                        } else {
                            0
                        },
                        entries: self.logs[(next_index - 1)..].to_vec(),
                        leader_commit: self.state.commit_index,
                    },
                )
                .await;
            }
        } else {
            unreachable!();
        }
    }

    /// Updates current_term, leader_id and sets role to Follower
    async fn demote_to_follower(&mut self, leader_id: Option<NodeId>, term: usize) {
        if let Role::Leader(leader) = &mut self.role {
            info!("Server {} is now a peasant follower :'(", self.id);

            let mut waiters = VecDeque::new();
            std::mem::swap(&mut leader.waiters, &mut waiters);

            for waiter in waiters {
                let resp = Err(ClientResponseError::WrongLeader(leader_id));

                self.emit(waiter.client_id, MessageContent::ClientResponse(resp))
                    .await;
            }
        }

        self.current_term = term;
        self.leader_id = leader_id;
        self.role = Role::Follower;
    }

    fn add_log(&mut self, mutation: StateMutation) {
        info!(
            "Leader {} adding log {} with mutation {:?}",
            self.id,
            self.logs.len() + 1,
            &mutation
        );
        self.logs.push(LogEntry {
            term: self.current_term,
            mutation,
        })
    }

    async fn notify_waiters(&mut self) {
        if let Role::Leader(leader) = &mut self.role {
            // We need to store waiters in a separate vector to prevent borrowing issues :dead:
            let mut ready_waiters = Vec::new();
            while let Some(waiter) = leader.waiters.front() {
                if waiter.index <= self.state.commit_index {
                    ready_waiters.push(leader.waiters.pop_front().unwrap());
                } else {
                    break;
                }
            }

            for waiter in ready_waiters {
                let resp = if waiter.term == self.logs[waiter.index - 1].term {
                    Ok(waiter.result)
                } else {
                    Err(ClientResponseError::EntryOverridden)
                };

                // We subtract node_count because our client index are after the servers
                info!(
                    "Server {} sending response to client {}",
                    self.id,
                    waiter.client_id - self.node_count
                );
                self.emit(waiter.client_id, MessageContent::ClientResponse(resp))
                    .await;
            }
        }
    }
}
