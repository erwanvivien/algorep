use std::{cmp::min, fs::OpenOptions};

use super::role::Role;
use crate::message::{Message, MessageContent};

use super::Node;

/// Message Handling part
impl Node {
    /// Handles every message exec from servers, returns true if we need to reset the election timeout
    pub(super) async fn handle_server_message(&mut self, message: Message) -> bool {
        let Message { from, content } = message;

        match content {
            MessageContent::VoteRequest {
                last_log_index,
                last_log_term,
                term,
            } => {
                let granted = term > self.current_term
                    && last_log_term >= self.logs.last().map(|e| e.term).unwrap_or(0)
                    && last_log_index >= self.logs.len();

                if term > self.current_term {
                    self.demote_to_follower(None, term).await;
                }

                self.voted_for = if granted { Some(from) } else { None };

                self.emit(
                    from,
                    MessageContent::VoteResponse {
                        granted,
                        term: self.current_term,
                    },
                );
                granted
            }
            MessageContent::VoteResponse { granted, term } => {
                if term > self.current_term {
                    self.demote_to_follower(None, term).await;
                }

                if let Role::Candidate(candidate) = &mut self.role {
                    if self.current_term == term && granted {
                        candidate.votes += 1;
                        if candidate.votes > self.node_count / 2 {
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
                term,
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

                    let mut file = OpenOptions::new()
                        .write(true)
                        .create(true)
                        .truncate(true)
                        .open(format!("node_{}.entries", self.id));

                    if let Ok(file) = &mut file {
                        serde_cbor::to_writer(file, &self.logs).unwrap();
                    }
                }

                self.emit(
                    from,
                    MessageContent::AppendResponse {
                        success,
                        match_index: self.logs.len(),
                        term: self.current_term,
                    },
                );

                success
            }
            MessageContent::AppendResponse {
                match_index,
                success,
                term,
            } => {
                if term > self.current_term {
                    self.demote_to_follower(None, term).await;
                }

                if let Role::Leader(leader) = &mut self.role {
                    if success {
                        leader.match_index[from] = match_index;
                        leader.next_index[from] = match_index + 1;

                        // Recompute commit index
                        let mut match_indices = leader.match_index.clone();
                        match_indices[self.id] = self.logs.len();
                        match_indices.sort();
                        let new_commit_index = match_indices[self.node_count / 2];

                        if new_commit_index > self.state.commit_index
                            && self.logs[new_commit_index - 1].term == self.current_term
                        {
                            leader.match_index[self.id] = match_index;
                            leader.next_index[self.id] = match_index + 1;

                            self.state.commit_index = new_commit_index;
                        }
                    } else {
                        leader.next_index[from] -= 1;
                        // TODO: Send again immediately to improve performance
                    }
                    true
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
