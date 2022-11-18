use regex::Regex;

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

impl ClientCommand {
    pub fn parse_command(line: &str) -> Option<ClientCommand> {
        let re = Regex::new(r"^(?P<command>\w+)\s*(?P<args>.*)\s*$").unwrap();
        let caps = re.captures(line)?;
        let command = caps.name("command")?.as_str();
        let args = caps.name("args")?.as_str();

        match command.to_lowercase().as_str() {
            "load" => {
                let re = Regex::new(r"^(?P<filename>\w+)").unwrap();
                let caps = re.captures(args)?;
                let filename = caps.name("filename")?.as_str();

                Some(ClientCommand::Load {
                    filename: filename.to_string(),
                })
            }
            "append" => {
                let re = Regex::new(r"^(?P<uid>[^\s]+)\s*(?P<text>.*)$").unwrap();
                let caps = re.captures(args)?;
                let uid = caps.name("uid")?.as_str();
                let text = caps.name("text")?.as_str();

                Some(ClientCommand::Append {
                    uid: uid.to_string(),
                    text: text.to_string(),
                })
            }
            "delete" => {
                let re = Regex::new(r"^(?P<uid>\w+)$").unwrap();
                let caps = re.captures(args)?;
                let uid = caps.name("uid")?.as_str();

                Some(ClientCommand::Delete {
                    uid: uid.to_string(),
                })
            }
            "list" => Some(ClientCommand::List),
            "get" => {
                let re = Regex::new(r"^(?P<uid>[^\s]+)$").unwrap();
                let caps = re.captures(args)?;
                let uid = caps.name("uid")?.as_str();

                Some(ClientCommand::Get {
                    uid: uid.to_string(),
                })
            }
            _ => None,
        }
    }
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
