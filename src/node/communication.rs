use log::error;

use crate::message::{Message, MessageContent};

use super::{Node, NodeId};

/// Communication part (emit & broadcast)
impl Node {
    pub(super) async fn emit(&self, id: NodeId, content: MessageContent) {
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

    pub(super) async fn broadcast(&self, content: MessageContent) {
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