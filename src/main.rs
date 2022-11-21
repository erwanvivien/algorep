mod client;
mod config;
mod entry;
mod message;
mod node;

#[cfg(test)]
mod tests;

use std::collections::VecDeque;
use tokio::sync::mpsc;

use config::{Config, CONFIG};
use message::Message;
use node::Node;

use log::{error, info};

use crate::{
    client::Client,
    message::{ClientCommand, ReplAction},
    node::persistent_state::PersistentState,
};

#[tokio::main]
async fn main() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .init();

    info!("Starting servers...");

    let Config {
        node_count,
        channel_capacity,
        ..
    } = *CONFIG;

    let mut threads = Vec::with_capacity(node_count);
    let mut senders = Vec::with_capacity(node_count);
    let mut receivers = VecDeque::with_capacity(node_count);

    // Create communication channels between nodes
    for _ in 0..node_count {
        let (sender, receiver) = mpsc::channel::<Message>(channel_capacity);

        senders.push(sender);
        receivers.push_back(receiver);
    }

    // Create communication channels between clients and nodes
    let (client_sender, client_receiver) = mpsc::channel::<Message>(channel_capacity);
    senders.push(client_sender);

    // Spawn nodes as light threads
    for id in 0..node_count {
        // The sender endpoint can be copied
        let receiver = receivers.pop_front().unwrap();
        let senders = senders.clone();

        let child = tokio::spawn(async move {
            let mut node = Node::new(id, node_count, receiver, senders);

            // Load state from disk
            let persistent_state = PersistentState::from_file(id);
            if let Some(persistent_state) = persistent_state {
                node.update_persistent(&persistent_state);
            }

            node.run().await;
        });

        threads.push(child);
    }

    // Instantiate client struct
    let client_id = node_count;
    let client = Client::new(client_id, node_count, client_receiver, senders.clone());

    // We leave the timer for scenarios where we want to test
    tokio::time::sleep(CONFIG.election_timeout_range().1).await;

    #[rustfmt::skip]
    println!(
r"
        Welcome to our RAFT implementation!
        Please see the README for more information.
        You have two ways to interact with the system:
          1. Use the REPL <server_id> <command> [args]
          2. Use the <command> [args]
"
    );

    // Read commands from stdin
    loop {
        let mut buffer = String::new();
        // We leave the timer for scenarios where we want to test
        tokio::time::sleep(CONFIG.election_timeout_range().1 * 2).await;

        let res = std::io::stdin().read_line(&mut buffer);
        if let Ok(count) = res {
            // Parse the stdin input and process it
            if count == 0 {
                info!("End of stream");
                break;
            }

            // Handles REPL commands: REPL <id> <command> [args]
            if let Some((id, repl)) = ReplAction::parse_action(&buffer) {
                senders[id]
                    .send(Message {
                        content: message::MessageContent::Repl(repl),
                        from: usize::MAX,
                    })
                    .await
                    .unwrap();
            // Handles client commands: <command> [args]
            } else if let Some(command) = ClientCommand::parse_command(&buffer) {
                info!("Parsed command: {:?}", &command);
                client.send_command(command).await;
            } else {
                error!("Failed to parse \"{}\"", &buffer);
            }
        } else if let Err(e) = res {
            error!("Failed to read line: {e}");
            break;
        }
    }

    // Send shutdown signal to all nodes after end of stream
    for sender in senders {
        let _ = sender
            .send(Message {
                content: message::MessageContent::Repl(ReplAction::Shutdown),
                from: usize::MAX,
            })
            .await;
    }

    // Wait for all nodes to shutdown gracefully
    for thread in threads.into_iter() {
        let _ = thread.await;
    }
}
