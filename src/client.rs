use std::sync::{
    atomic::{AtomicUsize, Ordering::Relaxed},
    Arc,
};

use tokio::{
    sync::mpsc::{Receiver, Sender},
    task::JoinHandle,
};

use crate::message::{ClientCommand, ClientResponseError, Message, MessageContent};

use log::{error, info};

/// Client struct is what is used to send commands to the cluster
pub struct Client {
    id: usize,
    senders: Vec<Sender<Message>>,
    leader_id: Arc<AtomicUsize>,
    receiver_handle: JoinHandle<()>,
}

impl Drop for Client {
    fn drop(&mut self) {
        self.receiver_handle.abort();
    }
}

impl Client {
    pub fn new(
        id: usize,
        node_count: usize,
        receiver: Receiver<Message>,
        senders: Vec<Sender<Message>>,
    ) -> Self {
        let leader_id = Arc::new(AtomicUsize::new(rand::random::<usize>() % node_count));
        let receiver_handle = Client::start_receiver(receiver, leader_id.clone());

        info!(
            "Client {} created with random leader {}",
            id - node_count,
            leader_id.load(Relaxed)
        );
        Self {
            id,
            senders,
            leader_id,
            receiver_handle,
        }
    }

    pub async fn send_command(&self, command: ClientCommand) {
        let leader_id = self.leader_id.load(Relaxed);
        let message_content = MessageContent::ClientRequest(command);

        let message = Message {
            content: message_content,
            from: self.id,
        };

        if let Err(err) = self.senders[leader_id].send(message).await {
            error!("Failed to send command: {}", err);
        }
    }

    pub fn start_receiver(
        receiver: Receiver<Message>,
        leader_id: Arc<AtomicUsize>,
    ) -> JoinHandle<()> {
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
                        } else {
                            error!("Error: {:?}", err);
                        }
                    }
                    _ => {}
                }
            }
        })
    }
}
