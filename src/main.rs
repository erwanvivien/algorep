use serde::{Deserialize, Serialize};

use std::sync::mpsc::{self, RecvError};
use std::sync::mpsc::{Receiver, Sender};
use std::thread;
use std::time::Duration;

#[derive(Debug, Deserialize, Serialize)]
enum ConfigTime {
    Second(u64),
    Millisecond(u64),
    Microsecond(u64),
    Nanosecond(u64),
    Minute(u64),
}

impl ConfigTime {
    #[allow(dead_code)]
    fn to_duration(self) -> Duration {
        match self {
            Self::Second(n) => Duration::from_secs(n),
            Self::Millisecond(n) => Duration::from_millis(n),
            Self::Microsecond(n) => Duration::from_micros(n),
            Self::Nanosecond(n) => Duration::from_nanos(n),
            Self::Minute(n) => Duration::from_secs(n * 60),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct Config {
    servers: usize,
    election_timeout: (ConfigTime, ConfigTime),
}

type ServerId = usize;

#[derive(Debug)]
enum MessageContent<'a> {
    Vote(ServerId),
    Data(&'a str),
}

#[derive(Debug)]
struct Message<'a> {
    content: MessageContent<'a>,
    to: ServerId,
}

const CONFIG_STR: &'static str = include_str!("../config/config.ron");

fn main() {
    let config: Config = ron::from_str(CONFIG_STR).expect("");

    let Config {
        servers,
        election_timeout,
    } = config;
    let (tx, rx): (Sender<Message>, Receiver<Message>) = mpsc::channel();
    let mut threads = Vec::with_capacity(servers);

    for id in 0..servers {
        // The sender endpoint can be copied
        let thread_tx = tx.clone();

        // Each thread will send its id via the channel
        let child = thread::spawn(move || {
            // The thread takes ownership over `thread_tx`
            // Each thread queues a message in the channel
            let message = Message {
                content: MessageContent::Vote(0),
                to: (id + 1) % servers,
            };
            thread_tx.send(message).unwrap();
        });

        threads.push(child);
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
