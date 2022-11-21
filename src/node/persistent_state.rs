use std::fs::OpenOptions;
use std::io::{Error, ErrorKind};

use crate::entry::LogEntry;

use super::{Node, NodeId};

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]

pub struct PersistentState {
    id: NodeId,

    pub current_term: usize,
    pub voted_for: Option<NodeId>,
    pub logs: Vec<LogEntry>,
}

impl From<&Node> for PersistentState {
    fn from(node: &Node) -> Self {
        Self {
            id: node.id,
            current_term: node.current_term,
            voted_for: node.voted_for,
            logs: node.logs.clone(),
        }
    }
}

impl PersistentState {
    pub fn from_file(id: NodeId) -> Option<Self> {
        let file = OpenOptions::new()
            .read(true)
            .open(format!("node_{}.entries", id))
            .ok();

        file.and_then(|f| serde_cbor::from_reader(f).ok())
    }

    fn to_file(&self) -> Result<(), std::io::Error> {
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(format!("node_{}.entries", self.id))?;

        serde_cbor::to_writer(file, self).map_err(|e| Error::new(ErrorKind::Other, e))
    }

    pub fn save(node: &Node) -> Result<(), Error> {
        Self::from(node).to_file()
    }
}
