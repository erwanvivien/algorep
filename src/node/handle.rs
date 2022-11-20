use std::{cmp::min, collections::VecDeque, fs::OpenOptions, pin::Pin, time::Duration};

use log::{debug, error, info};
use serde::{Deserialize, Serialize};
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

use super::Node;

/// Message Handling part
impl Node {
    pub(super) fn display(&self) {
        println!("====================");
        println!("Server {} state:", self.id);
        println!("Current term: {}", self.current_term);
        println!("Voted for: {:?}", self.voted_for);
        println!("Leader id: {:?}", self.leader_id);
        println!("Role: {}", self.role.to_string());
        if let Role::Leader(leader) = &self.role {
            println!("  Next index: {:?}", leader.next_index);
            println!("  Match index: {:?}", leader.match_index);
        } else if let Role::Candidate(candidate) = &self.role {
            println!("  Votes: {}", candidate.votes);
        }
        println!("Logs: {:?}", self.logs.len());
        println!("State:");
        println!("  Commit index: {}", self.state.commit_index);
        println!("  Last applied: {}", self.state.last_applied);
        println!("====================");
    }

    /// Handles only MessageContent::Repl
    pub(super) async fn handle_repl(&mut self, action: ReplAction) {
        match action {
            ReplAction::Crash => {
                info!("Server {} is crashed, ignoring messages", self.id);
                self.simulate_crash = true;
            }
            ReplAction::Start => {
                info!("Server {} is up, resuming message reception", self.id);
                self.simulate_crash = false;
            }
            ReplAction::Shutdown => {
                info!("Server {} is shuting down", self.id);
                self.shutdown_requested = true;
            }
            ReplAction::Timeout => {
                info!("Server {} is timed-out, becoming leader", self.id);
                self.promote_leader().await;
            }
            ReplAction::Display => {
                info!("Server {} is displaying state", self.id);
                self.display();
            }
            ReplAction::Recovery => {
                info!("Server {} current state is now empty", self.id);

                self.role = Role::Follower;
                self.current_term = 0;
                self.voted_for = None;
                self.logs = Vec::new();

                self.leader_id = None;

                self.state = VolatileState::new();

                self.display();

                info!(
                    "Server {0} is now recovering, from `node_{0}.entries`",
                    self.id
                );

                let mut file = OpenOptions::new()
                    .write(true)
                    .create(true)
                    .truncate(true)
                    .open(format!("node_{}.entries", self.id));

                if let Ok(file) = &mut file {
                    self.logs = serde_cbor::from_reader(file).unwrap();
                    self.display();
                }
            }
            _ => todo!(),
        }
    }

    pub(super) async fn handle_client_command(&mut self, message: Message) {
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
    pub(super) async fn handle_message(&mut self, message: Message) -> bool {
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

                        let mut file = OpenOptions::new()
                            .write(true)
                            .create(true)
                            .truncate(true)
                            .open(format!("node_{}.entries", self.id));

                        if let Ok(file) = &mut file {
                            serde_cbor::to_writer(file, &self.logs).unwrap();
                        }
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
                        let new_commit_index = match_indices[self.node_count / 2];

                        if self.state.commit_index != new_commit_index {
                            dbg!(self.state.commit_index, new_commit_index);
                            dbg!(match_indices);
                        }

                        if new_commit_index > self.state.commit_index
                            && self.logs[new_commit_index - 1].term == self.current_term
                        {
                            leader.match_index[self.id] = match_index;
                            leader.next_index[self.id] = match_index + 1;

                            self.state.commit_index = new_commit_index;

                            let mut file = OpenOptions::new()
                                .write(true)
                                .create(true)
                                .truncate(true)
                                .open(format!("node_{}.entries", self.id));

                            if let Ok(file) = &mut file {
                                for entry in &self.logs {
                                    serde_cbor::to_writer(&mut *file, entry).unwrap();
                                }
                            }
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
            MessageContent::ClientRequest(_)
            | MessageContent::ClientResponse(_)
            | MessageContent::Repl(_) => false,
        }
    }
}
