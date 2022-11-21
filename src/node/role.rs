use std::collections::VecDeque;

use crate::{message::ClientResponse, node::ClientId};

#[derive(Debug, PartialEq, Eq)]
pub struct CandidateData {
    pub votes: usize,
}

#[derive(Debug, PartialEq, Eq)]
pub struct WaitingClient {
    pub client_id: ClientId,
    pub term: usize,
    pub index: usize,
    pub result: ClientResponse,
}

#[derive(Debug, PartialEq, Eq)]
pub struct LeaderData {
    pub next_index: Vec<usize>,
    pub match_index: Vec<usize>,

    pub waiters: VecDeque<WaitingClient>,
}

impl LeaderData {
    pub fn new(next_index: usize, node_count: usize) -> Self {
        Self {
            next_index: vec![next_index; node_count],
            match_index: vec![0; node_count],

            waiters: VecDeque::new(),
        }
    }
}

/// This enum stores the current role of a node with the associated data
/// This allows use to be sure that when we access the data, we are in the correct role
#[derive(Debug, PartialEq, Eq)]
pub enum Role {
    Follower,
    Candidate(CandidateData),
    Leader(LeaderData),
}

impl Role {
    #[allow(dead_code)]
    pub fn is_follower(&self) -> bool {
        matches!(self, Role::Follower)
    }

    pub fn is_candidate(&self) -> bool {
        matches!(self, Role::Candidate(_))
    }

    pub fn is_leader(&self) -> bool {
        matches!(self, Role::Leader(_))
    }

    pub fn to_string(&self) -> &'static str {
        match self {
            Role::Follower => "Follower",
            Role::Candidate(_) => "Candidate",
            Role::Leader(_) => "Leader",
        }
    }
}
