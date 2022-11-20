use log::debug;

use super::role::{Role, Waiter};
use crate::{
    entry::StateMutation,
    message::{ClientCommand, ClientResponse, ClientResponseError, Message, MessageContent},
};

use super::Node;

/// Message Handling part
impl Node {
    // TODO: pass content client and from
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
                            format!("{}-{}", self.current_term, self.logs.len() + 1);

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
}
