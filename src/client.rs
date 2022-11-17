use std::sync::{
    atomic::{AtomicUsize, Ordering::Relaxed},
    Arc,
};

use tokio::sync::mpsc::{Receiver, Sender};

use crate::message::{ClientCommand, ClientResponseError, Message, MessageContent};
use regex::Regex;

use log::{error, info};

pub struct Client {
    id: usize,
    senders: Vec<Sender<Message>>,

    leader_id: Arc<AtomicUsize>,
}

impl Client {
    pub fn new(
        id: usize,
        node_count: usize,
        receiver: Receiver<Message>,
        senders: Vec<Sender<Message>>,
    ) -> Self {
        let leader_id = Arc::new(AtomicUsize::new(rand::random::<usize>() % node_count));
        Client::start_receiver(receiver, leader_id.clone());

        Self {
            id,
            senders,
            leader_id,
        }
    }

    pub async fn send_command(&self, command: ClientCommand) {
        let leader_id = self.leader_id.load(Relaxed);
        let message_content = MessageContent::ClientRequest(command);

        let message = Message {
            content: message_content,
            from: self.id,
            term: 0,
        };

        if let Err(err) = self.senders[leader_id].send(message).await {
            error!("Failed to send command: {}", err);
        }
    }

    pub fn start_receiver(receiver: Receiver<Message>, leader_id: Arc<AtomicUsize>) {
        // TODO: cleanup receiver thread
        tokio::spawn(async move {
            let mut receiver = receiver;
            while let Some(message) = receiver.recv().await {
                match message.content {
                    MessageContent::ClientResponse(Ok(response)) => {
                        info!("Client received response: {:?}", response);
                    }
                    MessageContent::ClientResponse(Err(err)) => {
                        if let ClientResponseError::WrongLeader(Some(new_leader_id)) = err {
                            info!("Client updated leader: {:?}", new_leader_id);
                            leader_id.store(new_leader_id, Relaxed);
                        }
                        else {
                            error!("Error: {:?}", err);
                        }
                    }
                    _ => {}
                }
            }
        });
    }

    pub fn parse_command(line: &str) -> Option<ClientCommand> {
        let re = Regex::new(r"^(?P<command>\w+)\s*(?P<args>.*)\s*$").unwrap();
        let caps = re.captures(line)?;
        let command = caps.name("command")?.as_str();
        let args = caps.name("args")?.as_str();

        match command.to_lowercase().as_str() {
            "load" => {
                let re = Regex::new(r"^(?P<filename>\w+)").unwrap();
                let caps = re.captures(args)?;
                let filename = caps.name("filename")?.as_str();

                Some(ClientCommand::Load {
                    filename: filename.to_string(),
                })
            }
            "append" => {
                let re = Regex::new(r"^(?P<uid>[^\s]+)\s*(?P<text>.*)$").unwrap();
                let caps = re.captures(args)?;
                let uid = caps.name("uid")?.as_str();
                let text = caps.name("text")?.as_str();

                Some(ClientCommand::Append {
                    uid: uid.to_string(),
                    text: text.to_string(),
                })
            }
            "delete" => {
                let re = Regex::new(r"^(?P<uid>\w+)$").unwrap();
                let caps = re.captures(args)?;
                let uid = caps.name("uid")?.as_str();

                Some(ClientCommand::Delete {
                    uid: uid.to_string(),
                })
            }
            "list" => Some(ClientCommand::List),
            "get" => {
                let re = Regex::new(r"^(?P<uid>[^\s]+)$").unwrap();
                let caps = re.captures(args)?;
                let uid = caps.name("uid")?.as_str();

                Some(ClientCommand::Get {
                    uid: uid.to_string(),
                })
            }
            _ => None,
        }
    }
}
