use std::{cmp::min, pin::Pin, time::Duration};

use tokio::{
    sync::mpsc::{Receiver, Sender},
    time::Sleep,
};

use crate::{
    entry::{Action, Entry},
    message::{ClientResponseError, Message, MessageContent, ReplAction},
    role::{CandidateData, LeaderData, Role},
    CONFIG,
};

pub type NodeId = usize;

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
    logs: Vec<Entry>,

    // Volatile state (for all servers)
    commit_index: usize,
    last_applied: usize,
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

            commit_index: 0,
            last_applied: 0,
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
                    println!("Server {} received {:?}", self.id, msg);
                    if msg.is_none() {
                        continue;
                    }

                    let msg = msg.unwrap();
                    match msg {
                        Message { content: MessageContent::Repl(action), ..} => {
                            self.handle_repl(action)
                        },
                        Message { content: MessageContent::ClientRequest(_), .. } => {
                            self.handle_client_action(msg).await
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
                        println!("Server {} received error, Timeout", self.id);
                        self.start_election().await;
                    } else {
                        println!("Server {} sending heartbeat", self.id);
                        self.send_entries().await;
                    }
                }
            }
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
                        leader_commit: self.commit_index,
                    },
                )
                .await;
            }
        } else {
            unreachable!();
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
            _ => {}
        }
    }

    async fn handle_client_action(&mut self, message: Message) {
        if let Message {
            content: MessageContent::ClientRequest(action),
            from,
            ..
        } = message
        {
            match action.clone() {
                Action::Set { key, value } => {
                    if !self.role.is_leader() {
                        println!("Server {} received error, Not a leader", self.id);
                        self.emit(
                            from,
                            MessageContent::ClientResponse(Err(ClientResponseError::WrongLeader(
                                self.leader_id,
                            ))),
                        )
                        .await;

                        return;
                    }

                    let entry = Entry {
                        action,
                        term: self.current_term,
                    };
                    self.logs.push(entry);
                    self.send_entries().await;

                    // TODO: Queue response until entry is committed
                    self.emit(from, MessageContent::ClientResponse(Ok(String::from(key))))
                        .await;
                }
                Action::Get { key } => todo!(),
                Action::Delete { key } => todo!(),
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

                self.current_term = term;
                self.voted_for = if accept { Some(from) } else { None };
                self.role = Role::Follower;

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
                    self.current_term = term;
                    self.role = Role::Follower;
                    self.leader_id = Some(from);
                }

                let same_logs = prev_log_index == 0
                    || self.logs.get(prev_log_index - 1).map(|e| e.term) == Some(prev_log_term);

                let success = same_term && same_logs;
                if success {
                    if prev_log_index != 0 {
                        self.logs.truncate(prev_log_index - 1);
                    }
                    self.logs.extend(entries);

                    if leader_commit > self.commit_index {
                        self.commit_index = min(leader_commit, self.logs.len());
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

                        if new_commit_index > self.commit_index
                            && self.logs[new_commit_index - 1].term == self.current_term
                        {
                            self.commit_index = new_commit_index;
                        }
                    } else {
                        leader.next_index[from] -= 1;
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
            dbg!(err);
        }
    }

    async fn broadcast(&self, content: MessageContent) {
        let message = Message {
            content,
            term: self.current_term,
            from: self.id,
        };

        for (i, sender) in self.senders[..self.node_count].iter().enumerate() {
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
