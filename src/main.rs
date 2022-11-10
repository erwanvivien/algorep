mod config;
mod entry;
mod message;
mod node;

#[cfg(test)]
mod tests;

use std::collections::VecDeque;
use tokio::sync::mpsc;

use config::CONFIG;
use message::Message;
use node::Node;

#[tokio::main]
async fn main() {
    let node_count = CONFIG.node_count;

    let mut threads = Vec::with_capacity(node_count);
    let mut senders = Vec::with_capacity(node_count);
    let mut receivers = VecDeque::with_capacity(node_count);

    for _ in 0..node_count {
        let (sender, receiver) = mpsc::channel::<Message>(4096);

        senders.push(sender);
        receivers.push_back(receiver);
    }

    for id in 0..node_count {
        // The sender endpoint can be copied
        let receiver = receivers.pop_front().unwrap();
        let senders = senders.clone();

        let child = tokio::spawn(async move {
            let mut node = Node::new(id, node_count, receiver, senders);
            node.run().await;
        });

        threads.push(child);
    }

    for thread in threads.into_iter() {
        let _ = thread.await;
    }
}
