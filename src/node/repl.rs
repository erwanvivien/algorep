use std::fs::OpenOptions;

use log::info;

use super::{role::Role, volatile_state::VolatileState};
use crate::message::ReplAction;

use super::Node;

/// Message Handling part
impl Node {
    fn display(&self) {
        println!("====================");
        println!("Server {} state:", self.id);
        println!("Current term: {}", self.current_term);
        println!("Voted for: {:?}", self.voted_for);
        println!("Leader id: {:?}", self.leader_id);
        println!("Role: {}", self.role.to_string());
        if let Role::Leader(leader) = &self.role {
            println!("  Next index: {:?}", leader.next_index);
            println!("  Match index: {:?}", leader.match_index);
        } else if let Role::Candidate(candidate) = &self.role {
            println!("  Votes: {}", candidate.votes);
        }
        println!("Logs: {:?}", self.logs.len());
        println!("State:");
        println!("  Commit index: {}", self.state.commit_index);
        println!("  Last applied: {}", self.state.last_applied);
        println!("====================");
    }

    /// Handles only MessageContent::Repl
    pub(super) async fn handle_repl(&mut self, action: ReplAction) {
        match action {
            ReplAction::Crash => {
                info!("Server {} is crashed, ignoring messages", self.id);
                self.simulate_crash = true;
            }
            ReplAction::Start => {
                info!("Server {} is up, resuming message reception", self.id);
                self.simulate_crash = false;
            }
            ReplAction::Shutdown => {
                info!("Server {} is shuting down", self.id);
                self.shutdown_requested = true;
            }
            ReplAction::Timeout => {
                info!("Server {} is timed-out, becoming leader", self.id);
                self.promote_leader().await;
            }
            ReplAction::Display => {
                info!("Server {} is displaying state", self.id);
                self.display();
            }
            ReplAction::Recovery => {
                info!("Server {} current state is now empty", self.id);

                self.role = Role::Follower;
                self.current_term = 0;
                self.voted_for = None;
                self.logs = Vec::new();

                self.leader_id = None;

                self.state = VolatileState::new();

                self.display();

                info!(
                    "Server {0} is now recovering, from `node_{0}.entries`",
                    self.id
                );

                let mut file = OpenOptions::new()
                    .write(true)
                    .create(true)
                    .truncate(true)
                    .open(format!("node_{}.entries", self.id));

                if let Ok(file) = &mut file {
                    self.logs = serde_cbor::from_reader(file).unwrap();
                    self.display();
                }
            }
            ReplAction::Speed(_) => todo!(),
        }
    }
}
