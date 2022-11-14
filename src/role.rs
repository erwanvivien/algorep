#[derive(Debug, PartialEq, Eq)]
pub struct CandidateData {
    pub votes: usize,
}

#[derive(Debug, PartialEq, Eq)]
pub struct LeaderData {
    pub next_index: Vec<usize>,
    pub match_index: Vec<usize>,
}

impl LeaderData {
    pub fn new(next_index: usize, node_count: usize) -> Self {
        Self {
            next_index: vec![next_index; node_count],
            match_index: vec![0; node_count],
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
