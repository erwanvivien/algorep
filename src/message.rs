use crate::node::NodeId;

#[allow(dead_code)]
#[derive(Debug, PartialEq, PartialOrd, Eq, Ord, Clone)]
pub enum MessageContent {
    Vote(NodeId),
    Data(String),
}

#[derive(Debug)]
pub struct Message {
    pub content: MessageContent,
    pub from: NodeId,
}
