use std::{cmp::min, collections::VecDeque, fs::OpenOptions, pin::Pin, time::Duration};

use log::{debug, error, info};
use serde::{Deserialize, Serialize};
use tokio::{
    sync::mpsc::{Receiver, Sender},
    time::Sleep,
};

use crate::{
    entry::{LogEntry, StateMutation},
    message::{
        ClientCommand, ClientResponse, ClientResponseError, Message, MessageContent, ReplAction,
    },
    role::{CandidateData, LeaderData, Role, Waiter},
    state::VolatileState,
    CONFIG,
};

use super::Node;

impl Node {
    pub(super) async fn start_election(&mut self) {
        info!("Server {} started election...", self.id);
        self.role = Role::Candidate(CandidateData { votes: 1 });
        self.current_term += 1;
        self.voted_for = Some(self.id);
        self.broadcast(MessageContent::VoteRequest {
            last_log_index: self.logs.len(),
            last_log_term: self.logs.last().map(|e| e.term).unwrap_or(0),
        })
        .await;
    }

    pub(super) async fn promote_leader(&mut self) {
        info!("Server {} is now leader !", self.id);
        self.role = Role::Leader(LeaderData::new(self.logs.len() + 1, self.node_count));

        self.leader_id = Some(self.id);
        self.send_entries().await;
    }
}
