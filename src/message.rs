use crate::{
    entry::{Action, Entry},
    node::NodeId,
};

#[allow(dead_code)]
#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub enum ReplAction {
    Speed(f32),
    Crash,
    Start,
    Shutdown,
}

#[allow(dead_code)]
#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub enum ClientResponseError {
    KeyNotFound,
    WrongLeader(Option<NodeId>),
}

#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub enum MessageContent {
    // Election
    VoteRequest {
        last_log_index: usize,
        last_log_term: usize,
    },
    VoteResponse(bool),

    // Log replication
    AppendEntries {
        entries: Vec<Entry>,
        // leader_id: NodeId,
        prev_log_index: usize,
        prev_log_term: usize,
        leader_commit: usize,
    },
    AppendResponse {
        success: bool,
        match_index: usize,
    },

    // External action
    #[allow(dead_code)]
    Repl(ReplAction),
    ClientRequest(Action),
    ClientResponse(Result<String, ClientResponseError>),
}

#[derive(Debug, Clone)]
pub struct Message {
    pub content: MessageContent,
    pub from: NodeId,
    // TODO: move append entries / vote_request
    pub term: usize,
}
