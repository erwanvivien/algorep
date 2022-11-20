use std::{cmp::min, fs::OpenOptions};

use crate::{
    message::{Message, MessageContent},
    role::Role,
};

use super::Node;

/// Message Handling part
impl Node {
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

                        // TODO(fix): maybe only if success
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
