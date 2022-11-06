mod config;
mod message;
mod node;

use std::collections::VecDeque;
use std::sync::mpsc;
use std::thread;

use config::Config;
use message::Message;
use node::Node;

use once_cell::sync::Lazy;

pub static CONFIG: Lazy<Config> =
    Lazy::new(|| ron::from_str(include_str!("../config/config.ron")).expect("Invalid config file"));

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
            let node = Node::new(id, receiver, senders);
            node.run();
        });

        threads.push(child);
    }

    for thread in threads {
        thread.join().unwrap();
    }
}
