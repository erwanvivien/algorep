use crate::server::ServerId;

#[derive(Debug)]
pub enum MessageContent<'a> {
    Vote(ServerId),
    Data(&'a str),
}

#[derive(Debug)]
pub struct Message<'a> {
    pub content: MessageContent<'a>,
    pub to: ServerId,
}
