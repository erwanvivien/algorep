mod config;
mod message;
mod server;

use std::sync::mpsc::{self, RecvError};
use std::sync::mpsc::{Receiver, Sender};
use std::thread;

use config::Config;
use message::{Message, MessageContent};
use server::Server;

const CONFIG_STR: &'static str = include_str!("../config/config.ron");

fn main() {
    let config: Config = ron::from_str(CONFIG_STR).expect("");
    let Config { servers, .. } = config;

    let (tx, rx): (Sender<Message>, Receiver<Message>) = mpsc::channel();
    let mut threads = Vec::with_capacity(servers);

    for id in 0..servers {
        // The sender endpoint can be copied
        let thread_tx = tx.clone();

        let child = thread::spawn(move || {
            let message = Message {
                content: MessageContent::Vote(0),
                to: id,
            };
            thread_tx.send(message).unwrap();
        });

        threads.push(Server { thread: child, id });
    }

    // Here, all the messages are collected
    loop {
        let content = rx.recv();
        match content {
            Ok(data) => println!("{:?}", data),
            Err(RecvError) => (),
        }
    }
}
