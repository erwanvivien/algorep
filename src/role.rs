use std::collections::VecDeque;

use crate::node::ClientId;

#[derive(Debug, PartialEq, Eq)]
pub struct CandidateData {
    pub votes: usize,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Waiter {
    pub client_id: ClientId,
    pub term: usize,
    pub index: usize,
    pub filename: String,
}

#[derive(Debug, PartialEq, Eq)]
pub struct LeaderData {
    pub next_index: Vec<usize>,
    pub match_index: Vec<usize>,

    pub waiters: VecDeque<Waiter>,
}

impl LeaderData {
    pub fn new(next_index: usize, node_count: usize) -> Self {
        Self {
            next_index: vec![next_index; node_count],
            match_index: vec![0; node_count],

            waiters: VecDeque::new()
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Role {
    Follower,
    Candidate(CandidateData),
    Leader(LeaderData),
}

#[allow(dead_code)]
impl Role {
    pub fn is_follower(&self) -> bool {
        matches!(self, Role::Follower)
    }

    pub fn is_candidate(&self) -> bool {
        matches!(self, Role::Candidate(_))
    }

    pub fn is_leader(&self) -> bool {
        matches!(self, Role::Leader(_))
    }
}
