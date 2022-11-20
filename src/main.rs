mod client;
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

use log::{error, info};

use crate::{
    client::Client,
    message::{ClientCommand, ReplAction},
};

#[tokio::main]
async fn main() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .init();

    info!("Starting servers...");

    let node_count = CONFIG.node_count;

    let mut threads = Vec::with_capacity(node_count);
    let mut senders = Vec::with_capacity(node_count);
    let mut receivers = VecDeque::with_capacity(node_count);

    for _ in 0..node_count {
        let (sender, receiver) = mpsc::channel::<Message>(4096);

        senders.push(sender);
        receivers.push_back(receiver);
    }

    let client_count = 1;
    for _ in 0..client_count {
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

    let client_id = node_count;
    let client = Client::new(
        client_id,
        node_count,
        receivers.pop_front().unwrap(),
        senders.clone(),
    );

    tokio::time::sleep(CONFIG.election_timeout_range().1).await;

    loop {
        let mut buffer = String::with_capacity(100);
        tokio::time::sleep(CONFIG.election_timeout_range().1 * 2).await;

        let res = std::io::stdin().read_line(&mut buffer);
        if let Ok(count) = res {
            // parse line
            if count == 0 {
                info!("End of stream");
                break;
            }

            if let Some((id, repl)) = ReplAction::parse_action(&buffer) {
                senders[id]
                    .send(Message {
                        content: message::MessageContent::Repl(repl),
                        from: usize::MAX,
                        term: usize::MAX,
                    })
                    .await
                    .unwrap();
            } else if let Some(command) = ClientCommand::parse_command(&buffer) {
                info!("Parsed command: {:?}", &command);
                client.send_command(command).await;
            } else {
                error!("Failed to parse \"{}\"", &buffer);
            };
        } else if let Err(e) = res {
            error!("Failed to read line: {e}");
            break;
        }
    }

    for senders in senders {
        while let Err(_) = senders
            .send(Message {
                content: message::MessageContent::Repl(ReplAction::Shutdown),
                from: usize::MAX,
                term: usize::MAX,
            })
            .await
        {
            error!("Failed to send shutdown message");
        }
    }
    for thread in threads.into_iter() {
        let _ = thread.await;
    }
}
