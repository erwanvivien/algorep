use log::{debug, info};

use super::role::{Role, WaitingClient};
use crate::{
    entry::StateMutation,
    message::{ClientCommand, ClientResponse, ClientResponseError, MessageContent},
};

use super::Node;

/// Message Handling part
impl Node {
    /// Handles every message exec from clients
    pub(super) async fn handle_client_command(&mut self, command: ClientCommand, from: usize) {
        if let Role::Leader(leader) = &mut self.role {
            match command {
                ClientCommand::Load { filename } => {
                    let uid = format!("{}-{}", self.current_term, self.logs.len() + 1);

                    leader.waiters.push_back(WaitingClient {
                        client_id: from,
                        term: self.current_term,
                        index: self.logs.len() + 1,
                        result: ClientResponse::Uid(uid.clone()),
                    });

                    self.add_log(StateMutation::Create { filename, uid });
                }
                ClientCommand::List => {
                    let files = self.state.list_uid();

                    self.emit(
                        from,
                        MessageContent::ClientResponse(Ok(ClientResponse::List(files))),
                    );
                }
                ClientCommand::Delete { uid } => {
                    leader.waiters.push_back(WaitingClient {
                        client_id: from,
                        term: self.current_term,
                        index: self.logs.len() + 1,
                        result: ClientResponse::Ok,
                    });

                    self.add_log(StateMutation::Delete { uid });
                }
                ClientCommand::Append { uid, text } => {
                    leader.waiters.push_back(WaitingClient {
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
                }
            }
        } else {
            debug!("Server {} received error, Not a leader", self.id);
            self.emit(
                from,
                MessageContent::ClientResponse(Err(ClientResponseError::WrongLeader(
                    self.leader_id,
                ))),
            );
        }
    }

    /// Notify clients of the result of their command
    pub(super) async fn notify_waiting_clients(&mut self) {
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
                self.emit(waiter.client_id, MessageContent::ClientResponse(resp));
            }
        }
    }
}
