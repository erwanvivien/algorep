use std::{
    fmt::{Display, Formatter},
    time::Duration,
};

use regex::Regex;

use crate::{entry::LogEntry, node::volatile_state::File, node::NodeId};

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Message {
    pub content: MessageContent,
    pub from: NodeId,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum MessageContent {
    // Election
    VoteRequest {
        last_log_index: usize,
        last_log_term: usize,
        term: usize,
    },
    VoteResponse {
        granted: bool,
        term: usize,
    },

    // Log replication
    AppendEntries {
        entries: Vec<LogEntry>,
        // leader_id: NodeId,
        prev_log_index: usize,
        prev_log_term: usize,
        leader_commit: usize,
        term: usize,
    },
    AppendResponse {
        success: bool,
        match_index: usize,
        term: usize,
    },

    // Client actions
    ClientRequest(ClientCommand),
    ClientResponse(Result<ClientResponse, ClientResponseError>),

    // External action
    Repl(ReplAction),
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Clone)]
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
    Uid(String),
    List(Vec<String>),
    File(File),
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Clone)]
pub enum ClientResponseError {
    EntryOverridden,
    FileNotFound,
    WrongLeader(Option<NodeId>),
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

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum ReplAction {
    Speed(Speed),
    Crash,
    Start,
    Recovery,

    // Personnal commands
    Shutdown,
    Timeout,
    Display,
}

impl ReplAction {
    pub fn parse_action(action: &str) -> Option<(NodeId, ReplAction)> {
        let action = action.to_lowercase();

        let repl_id = Regex::new(r"^repl (?P<id>\d+)").unwrap();
        if !repl_id.is_match(&action) {
            return None;
        }

        let id_capture = repl_id.captures(&action)?;
        let id = id_capture.name("id")?.as_str().parse::<NodeId>().ok()?;

        let speed_re = Regex::new(r"speed (?P<speed>\w+)").unwrap();
        if let Some(caps) = speed_re.captures(&action) {
            let speed = caps.name("speed")?.as_str();

            let speed = match speed {
                "fast" => Some(Speed::Fast),
                "medium" => Some(Speed::Medium),
                "slow" => Some(Speed::Slow),
                _ => return None,
            };

            speed.map(|fast| (id, ReplAction::Speed(fast)))
        } else if action.contains("crash") {
            Some((id, ReplAction::Crash))
        } else if action.contains("start") {
            Some((id, ReplAction::Start))
        } else if action.contains("shutdown") {
            Some((id, ReplAction::Shutdown))
        } else if action.contains("recovery") {
            Some((id, ReplAction::Recovery))
        } else if action.contains("timeout") {
            Some((id, ReplAction::Timeout))
        } else if action.contains("display") {
            Some((id, ReplAction::Display))
        } else {
            None
        }
    }
}

#[derive(Debug, Copy, PartialEq, Eq, Clone)]
pub enum Speed {
    Fast,
    Medium,
    Slow,
}

impl Display for Speed {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Speed::Fast => write!(f, "Fast"),
            Speed::Medium => write!(f, "Medium"),
            Speed::Slow => write!(f, "Slow"),
        }
    }
}

impl From<Speed> for Duration {
    fn from(speed: Speed) -> Self {
        match speed {
            Speed::Fast => Duration::from_millis(0),
            Speed::Medium => Duration::from_millis(100),
            Speed::Slow => Duration::from_millis(1000),
        }
    }
}
