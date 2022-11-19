use serde::{Deserialize, Serialize};

#[allow(dead_code)]
#[derive(Debug, PartialEq, PartialOrd, Clone, Serialize, Deserialize)]
pub enum StateMutation {
    Create { uid: String, filename: String },
    Delete { uid: String },
    Append { uid: String, text: String },
}

#[derive(Debug, PartialEq, PartialOrd, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub term: usize,
    pub mutation: StateMutation,
}
