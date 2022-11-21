use crate::message::{Message, MessageContent};

use super::{Node, NodeId};

/// Communication part (emit & broadcast)
impl Node {
    pub(super) fn emit(&self, id: NodeId, content: MessageContent) {
        let sender = self.senders[id].clone();
        let from = self.id;

        tokio::spawn(async move { sender.send(Message { content, from }).await });
    }

    pub(super) fn broadcast(&self, content: MessageContent) {
        let message = Message {
            content,
            from: self.id,
        };

        for (i, sender) in self.senders[..self.node_count].iter().enumerate() {
            if i == self.id {
                continue;
            }

            let sender = sender.clone();
            let message = message.clone();

            tokio::spawn(async move { sender.send(message).await });
        }
    }
}
