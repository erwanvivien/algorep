mod config;
mod message;
mod node;

use std::collections::VecDeque;
use std::sync::mpsc::{self, RecvError};
use std::thread;

use config::Config;
use message::{Message, MessageContent};
use node::Node;

const CONFIG_STR: &'static str = include_str!("../config/config.ron");

fn main() {
    let config: Config = ron::from_str(CONFIG_STR).expect("");
    let Config { servers, .. } = config;

    let mut threads = Vec::with_capacity(servers);
    let mut senders = Vec::with_capacity(servers);
    let mut receivers = VecDeque::with_capacity(servers);

    for _ in 0..servers {
        let (sender, receiver) = mpsc::channel::<Message>();

        senders.push(sender);
        receivers.push_back(receiver);
    }

    for id in 0..servers {
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
