use std::{cmp::min, collections::VecDeque, pin::Pin, time::Duration};

use log::{debug, error, info};
use tokio::{
    sync::mpsc::{Receiver, Sender},
    time::Sleep,
};

use crate::{
    entry::{LogEntry, StateMutation},
    message::{
        ClientCommand, ClientResponse, ClientResponseError, Message, MessageContent, ReplAction,
    },
    role::{CandidateData, LeaderData, Role, Waiter},
    state::VolatileState,
    CONFIG,
};

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

    // Persistent state (for all server)
    role: Role,
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

            tokio::select! {
                msg = self.receiver.recv() => {
                    debug!("Server {} received {:?}", self.id, msg);
                    if msg.is_none() {
                        continue;
                    }

                    let msg = msg.unwrap();
                    match msg {
                        Message { content: MessageContent::Repl(action), ..} => {
                            self.handle_repl(action)
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

    fn generate_timeout(&self) -> Pin<Box<Sleep>> {
        let duration = {
            let (min, max) = self.election_timeout_range;

            if self.role.is_leader() {
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

    async fn start_election(&mut self) {
        info!("Server {} started election...", self.id);
        self.role = Role::Candidate(CandidateData { votes: 1 });
        self.current_term += 1;
        self.voted_for = Some(self.id);
        self.broadcast(MessageContent::VoteRequest {
            last_log_index: self.logs.len(),
            last_log_term: self.logs.last().map(|e| e.term).unwrap_or(0),
        })
        .await;
    }

    async fn promote_leader(&mut self) {
        info!("Server {} is now leader !", self.id);
        self.role = Role::Leader(LeaderData::new(self.logs.len() + 1, self.node_count));

        self.leader_id = Some(self.id);
        self.send_entries().await;
    }

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
        info!("Leader {} adding log {} with mutation {:?}", self.id, self.logs.len() + 1, &mutation);
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
                info!("Server {} sending response to client {}", self.id, waiter.client_id - self.node_count);
                self.emit(waiter.client_id, MessageContent::ClientResponse(resp))
                    .await;
            }
        }
    }
}

/// Message Handling part
impl Node {
    /// Handles only MessageContent::Repl
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
            // ReplAction::Recovery => {
            //     self.state = VolatileState::new();
            // }
            _ => todo!(),
        }
    }

    async fn handle_client_command(&mut self, message: Message) {
        if let Message {
            content: MessageContent::ClientRequest(command),
            from,
            ..
        } = message
        {
            if let Role::Leader(leader) = &mut self.role {
                match command.clone() {
                    ClientCommand::Load { filename } => {
                        let uid =
                            format!("{}-{}", self.current_term, self.logs.len() + 1).to_string();

                        leader.waiters.push_back(Waiter {
                            client_id: from,
                            term: self.current_term,
                            index: self.logs.len() + 1,
                            result: ClientResponse::UID(uid.clone()),
                        });

                        self.add_log(StateMutation::Create { filename, uid });
                    }
                    ClientCommand::List => {
                        let files = self.state.list_uid();

                        self.emit(
                            from,
                            MessageContent::ClientResponse(Ok(ClientResponse::List(files))),
                        )
                        .await;
                    }
                    ClientCommand::Delete { uid } => {
                        leader.waiters.push_back(Waiter {
                            client_id: from,
                            term: self.current_term,
                            index: self.logs.len() + 1,
                            result: ClientResponse::Ok,
                        });

                        self.add_log(StateMutation::Delete { uid });
                    }
                    ClientCommand::Append { uid, text } => {
                        leader.waiters.push_back(Waiter {
                            client_id: from,
                            term: self.current_term,
                            index: self.logs.len() + 1,
                            result: ClientResponse::Ok,
                        });

                        self.add_log(StateMutation::Append { uid, text });
                    }
                    ClientCommand::Get { uid } => {
                        let file = self.state.get(&uid);

                        self.emit(
                            from,
                            MessageContent::ClientResponse(if let Some(file) = file {
                                Ok(ClientResponse::File(file.clone()))
                            } else {
                                Err(ClientResponseError::FileNotFound)
                            }),
                        )
                        .await
                    }
                }
            } else {
                debug!("Server {} received error, Not a leader", self.id);
                self.emit(
                    from,
                    MessageContent::ClientResponse(Err(ClientResponseError::WrongLeader(
                        self.leader_id,
                    ))),
                )
                .await;
            }
        }
    }

    /// Handles every message exec MessageContent::Repl
    async fn handle_message(&mut self, message: Message) -> bool {
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

                self.voted_for = if accept { Some(from) } else { None };
                self.demote_to_follower(None, term).await;

                self.emit(from, MessageContent::VoteResponse(accept)).await;
                accept
            }
            MessageContent::VoteResponse(granted) => {
                if let Role::Candidate(candidate) = &mut self.role {
                    if self.current_term == term && granted {
                        candidate.votes += 1;
                        if candidate.votes > self.senders.len() / 2 {
                            self.promote_leader().await;
                        }
                    }
                }

                false
            }
            MessageContent::AppendEntries {
                entries,
                prev_log_index,
                prev_log_term,
                leader_commit,
            } => {
                let same_term = term >= self.current_term;

                if same_term {
                    self.demote_to_follower(Some(from), term).await;
                }

                let same_logs = prev_log_index == 0
                    || self.logs.get(prev_log_index - 1).map(|e| e.term) == Some(prev_log_term);

                let success = same_term && same_logs;
                if success {
                    self.logs.truncate(prev_log_index);
                    self.logs.extend(entries);

                    if leader_commit > self.state.commit_index {
                        self.state.commit_index = min(leader_commit, self.logs.len());
                    }
                }

                self.emit(
                    from,
                    MessageContent::AppendResponse {
                        success,
                        match_index: self.logs.len(),
                    },
                )
                .await;

                success
            }
            MessageContent::AppendResponse {
                match_index,
                success,
            } => {
                if let Role::Leader(leader) = &mut self.role {
                    if success {
                        leader.match_index[from] = match_index;
                        leader.next_index[from] = match_index + 1;

                        // Recompute commit index
                        let mut match_indices = leader.match_index.clone();
                        match_indices[self.id] = self.logs.len();
                        match_indices.sort();
                        let new_commit_index = match_indices[self.senders.len() / 2];

                        if new_commit_index > self.state.commit_index
                            && self.logs[new_commit_index - 1].term == self.current_term
                        {
                            self.state.commit_index = new_commit_index;
                        }
                    } else {
                        leader.next_index[from] -= 1;
                        // TODO: Send again immediately
                    }
                    return true;
                } else {
                    false
                }
            }
            // Repl case is handled in `handle_repl`
            _ => false,
        }
    }
}

/// Communication part (emit & broadcast)
impl Node {
    async fn emit(&self, id: NodeId, content: MessageContent) {
        let res = self.senders[id]
            .send(Message {
                content,
                term: self.current_term,
                from: self.id,
            })
            .await;

        if let Err(err) = res {
            error!("{err}");
        }
    }

    async fn broadcast(&self, content: MessageContent) {
        // TODO: Fix parallel futures
        let message = Message {
            content,
            term: self.current_term,
            from: self.id,
        };

        let mut futures = Vec::with_capacity(self.senders.len() - 1);

        for (i, sender) in self.senders[..self.node_count].iter().enumerate() {
            if i == self.id {
                continue;
            }

            let sender = sender.clone();
            let message = message.clone();

            futures.push(tokio::spawn(async move { sender.send(message).await }));
        }

        for fut in futures {
            let res = fut.await;
            if let Err(err) = res {
                error!("{err}");
            }
        }
    }
}
