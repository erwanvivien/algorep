use log::info;

use super::{role::Role, volatile_state::VolatileState};
use crate::message::ReplAction;

use super::persistent_state::PersistentState;
use super::Node;

/// Message Handling part
impl Node {
    /// Display server state
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

                self.state = VolatileState::new();
                self.logs.clear();
                self.current_term = 0;
                self.leader_id = None;
                self.voted_for = None;
            }
            ReplAction::Start => {
                info!("Starting clients");
                // We are playing the role of clients, so we need do not need to
                // start the clients.
            }
            ReplAction::Shutdown => {
                info!("Server {} is shuting down", self.id);
                self.shutdown_requested = true;
            }
            ReplAction::Timeout => {
                info!("Server {} is timed-out, becoming leader", self.id);
                self.start_election().await;
            }
            ReplAction::Display => {
                info!("Server {} is displaying state", self.id);
                self.display();
            }
            ReplAction::Recovery => {
                info!("Server {} current state is now empty", self.id);

                let state = PersistentState::from_file(self.id);
                if let Some(state) = state {
                    info!(
                        "Server {0} is now recovering, from `node_{0}.entries`",
                        self.id
                    );

                    self.simulate_crash = false;

                    self.role = Role::Follower;
                    self.update_persistent(&state);

                    self.leader_id = None;

                    self.display();
                }
            }
            ReplAction::Speed(speed) => {
                info!("Server {} new speed is {speed}", self.id);

                self.speed = speed;
            }
        }
    }
}
