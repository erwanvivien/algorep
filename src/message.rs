use crate::node::NodeId;

#[allow(dead_code)]
#[derive(Debug, PartialEq, PartialOrd, Eq, Ord, Clone)]
pub enum MessageContent {
    VoteRequest,
    VoteResponse(bool),
    Heartbeat,
    Data(String),
}

#[derive(Debug)]
pub struct Message {
    pub content: MessageContent,
    pub from: NodeId,
    pub term: usize,
}
