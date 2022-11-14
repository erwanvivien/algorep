use crate::entry::{Action, Entry};
use std::collections::HashMap;

pub struct State {
    map: HashMap<String, String>,
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

    pub fn get(&self, key: &str) -> Option<&String> {
        self.map.get(key)
    }

    pub fn process(&mut self, action: &Action) {
        match action {
            Action::Set { key, value } => {
                self.map.insert(String::from(key), String::from(value));
            }
            Action::Append { key, value } => {
                let entry = self.map.entry(String::from(key)).or_insert(String::new());
                entry.push_str(value);
            }
            Action::Delete { key } => {
                self.map.remove(key);
            }
            Action::Get { .. } => (),
        }
    }

    pub fn process_batch(&mut self, entries: &[Entry]) {
        for entry in entries {
            self.process(&entry.action);
        }
    }

    pub fn apply_committed_entries(&mut self, logs: &Vec<Entry>) {
        if self.last_applied > 0 && self.commit_index > self.last_applied {
            // Apply
            let entries = &logs[(self.last_applied - 1)..(self.commit_index - 1)];
            self.process_batch(entries);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::State;
    use crate::entry::Action;

    #[test]
    fn test_append() {
        let mut state = State::new();
        state.process(&Action::Append {
            key: String::from("key"),
            value: String::from("value"),
        });
        assert_eq!(state.get("key"), Some(&String::from("value")));
    }
}
