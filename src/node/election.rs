use log::info;

use super::role::{CandidateData, LeaderData, Role};
use crate::{entry::StateMutation, message::MessageContent};

use super::Node;

impl Node {
    /// The function called on timer expiration
    pub(super) async fn start_election(&mut self) {
        info!("Server {} started election...", self.id);
        self.role = Role::Candidate(CandidateData { votes: 1 });
        self.current_term += 1;
        self.voted_for = Some(self.id);
        self.broadcast(MessageContent::VoteRequest {
            last_log_index: self.logs.len(),
            last_log_term: self.logs.last().map(|e| e.term).unwrap_or(0),
            term: self.current_term,
        });
    }

    /// The function called when a node is elected leader
    pub(super) async fn promote_leader(&mut self) {
        info!("Server {} is now leader !", self.id);
        self.role = Role::Leader(LeaderData::new(self.logs.len() + 1, self.node_count));

        self.leader_id = Some(self.id);
        self.add_log(StateMutation::Noop);

        self.send_entries().await;
    }
}
