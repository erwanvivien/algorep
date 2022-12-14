use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Eq, PartialOrd, Clone, Serialize, Deserialize)]
pub enum StateMutation {
    Create { uid: String, filename: String },
    Delete { uid: String },
    Append { uid: String, text: String },
    Noop,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub term: usize,
    pub mutation: StateMutation,
}
