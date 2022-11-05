use crate::server::ServerId;

#[derive(Debug, PartialEq, PartialOrd, Eq, Ord)]
pub enum MessageContent {
    Vote(ServerId),
    Data(String),
}

#[derive(Debug)]
pub struct Message {
    pub content: MessageContent,
    pub from: ServerId,
}
