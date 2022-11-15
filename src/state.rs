use crate::entry::{Entry, StateMutation};
use std::collections::HashMap;

#[derive(Debug, Eq, PartialEq, PartialOrd, Clone)]
pub struct File {
    pub filename: String,
    pub text: String,
}

pub struct State {
    map: HashMap<String, File>,
    last_applied: usize,
    pub commit_index: usize,
}

#[allow(dead_code)]
impl State {
    pub fn new() -> Self {
        State {
            map: HashMap::new(),
            commit_index: 0,
            last_applied: 0,
        }
    }

    pub fn get(&self, uid: &str) -> Option<&File> {
        self.map.get(uid)
    }

    pub fn process(&mut self, action: &StateMutation) {
        match action {
            StateMutation::Create { uid, filename } => {
                self.map.insert(
                    uid.clone(),
                    File {
                        filename: filename.clone(),
                        text: String::new(),
                    },
                );
            }
            StateMutation::Delete { uid } => {
                self.map.remove(uid);
            }
            StateMutation::Append { uid, text } => {
                if let Some(file) = self.map.get_mut(uid) {
                    file.text.push_str(text);
                }
            }
        }
    }

    pub fn process_batch(&mut self, entries: &[Entry]) {
        for entry in entries {
            self.process(&entry.action);
        }
    }

    pub fn apply_committed_entries(&mut self, logs: &Vec<Entry>) {
        if self.commit_index > self.last_applied {
            // Apply
            let entries = &logs[self.last_applied..self.commit_index];
            self.process_batch(entries);
            self.last_applied = self.commit_index;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::State;
    use crate::entry::StateMutation;

    #[test]
    fn test_append() {
        let mut state = State::new();
        state.process(&StateMutation::Create {
            uid: "1".to_string(),
            filename: "file1".to_string(),
        });
        state.process(&StateMutation::Append {
            uid: "1".to_string(),
            text: "hello".to_string(),
        });

        assert_eq!(state.get("1").unwrap().text, "hello");
    }
}
