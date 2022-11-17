use crate::{entry::LogEntry, node::NodeId, state::File};

#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub enum ReplAction {
    Speed(f32),
    Crash,
    Start,
    Shutdown,
    Recovery,
}

#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub enum ClientCommand {
    Load { filename: String },
    List,
    Delete { uid: String },
    Append { uid: String, text: String },
    Get { uid: String },
}

#[derive(Debug, Eq, PartialEq, PartialOrd, Clone)]
pub enum ClientResponse {
    Ok,
    UID(String),
    List(Vec<String>),
    File(File),
}

#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub enum ClientResponseError {
    EntryOverridden,
    FileNotFound,
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
        entries: Vec<LogEntry>,
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
    ClientRequest(ClientCommand),
    ClientResponse(Result<ClientResponse, ClientResponseError>),
}

#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub struct Message {
    pub content: MessageContent,
    pub from: NodeId,
    // TODO: move append entries / vote_request
    pub term: usize,
}
