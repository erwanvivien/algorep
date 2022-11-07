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
    VoteRequest,
    VoteResponse(bool),
    // Log replication
    AppendEntries {
        logs: Vec<Entry>,
        leader_id: NodeId,
    },
    AppendResponse(bool),

    // External action
    #[allow(dead_code)]
    Repl(ReplAction),
}

#[derive(Debug)]
pub struct Message {
    pub content: MessageContent,
    pub from: NodeId,
    pub term: usize,
}
