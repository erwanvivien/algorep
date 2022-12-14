use crate::entry::{LogEntry, StateMutation};
use std::collections::HashMap;

#[derive(Debug, Eq, PartialEq, PartialOrd, Clone)]
pub struct File {
    pub filename: String,
    pub text: String,
}

/// The volatile state is a simple hashmap. The hashmap is in sync up-to commit_index for a specific node
#[derive(Debug)]
pub struct VolatileState {
    storage: HashMap<String, File>,
    pub last_applied: usize,
    pub commit_index: usize,
}

impl VolatileState {
    pub fn new() -> Self {
        VolatileState {
            storage: HashMap::new(),
            commit_index: 0,
            last_applied: 0,
        }
    }

    pub fn get(&self, uid: &str) -> Option<&File> {
        self.storage.get(uid)
    }

    pub fn list_uid(&self) -> Vec<String> {
        self.storage.keys().cloned().collect()
    }

    /// Apply a log entry to the volatile state
    pub fn process(&mut self, action: &StateMutation) {
        match action {
            StateMutation::Create { uid, filename } => {
                self.storage.insert(
                    uid.clone(),
                    File {
                        filename: filename.clone(),
                        text: String::new(),
                    },
                );
            }
            StateMutation::Delete { uid } => {
                self.storage.remove(uid);
            }
            StateMutation::Append { uid, text } => {
                if let Some(file) = self.storage.get_mut(uid) {
                    file.text.push_str(text);
                    file.text.push('\n');
                }
            }
            StateMutation::Noop => {}
        }
    }

    /// Apply commited log entries to the volatile state
    pub fn apply_committed_entries(&mut self, logs: &[LogEntry]) {
        if self.commit_index > self.last_applied {
            // Apply
            let entries = &logs[self.last_applied..self.commit_index];
            for entry in entries {
                self.process(&entry.mutation);
            }
            self.last_applied = self.commit_index;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::VolatileState;
    use crate::entry::StateMutation;

    #[test]
    fn test_append() {
        let mut state = VolatileState::new();
        state.process(&StateMutation::Create {
            uid: "1".to_string(),
            filename: "file1".to_string(),
        });
        state.process(&StateMutation::Append {
            uid: "1".to_string(),
            text: "hello".to_string(),
        });

        assert_eq!(state.get("1").unwrap().text, "hello\n");
    }
}
