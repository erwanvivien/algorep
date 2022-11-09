use crate::{entry::Entry, node::NodeId};

#[allow(dead_code)]
#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub enum ReplAction {
    Speed(f32),
    Crash,
    Start,
    Shutdown,
}

#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub enum MessageContent {
    VoteRequest {
        // Currently represent the log length, because it would be -1 on startup
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
    AppendResponse(bool),

    // External action
    #[allow(dead_code)]
    Repl(ReplAction),
}

#[derive(Debug, Clone)]
pub struct Message {
    pub content: MessageContent,
    pub from: NodeId,
    pub term: usize,
}
