mod config;
mod message;
mod node;

#[cfg(test)]
mod tests;

use std::collections::VecDeque;
use std::sync::mpsc;
use std::thread;

use config::CONFIG;
use message::Message;
use node::Node;


fn main() {
    let node_count = CONFIG.node_count;

    let mut threads = Vec::with_capacity(node_count);
    let mut senders = Vec::with_capacity(node_count);
    let mut receivers = VecDeque::with_capacity(node_count);

    for _ in 0..node_count {
        let (sender, receiver) = mpsc::channel::<Message>();

        senders.push(sender);
        receivers.push_back(receiver);
    }

    for id in 0..node_count {
        // The sender endpoint can be copied
        let receiver = receivers.pop_front().unwrap();
        let senders = senders.clone();

        let child = thread::spawn(move || {
            let mut node = Node::new(id, receiver, senders);
            node.run();
        });

        threads.push(child);
    }

    for thread in threads {
        thread.join().unwrap();
    }
}
