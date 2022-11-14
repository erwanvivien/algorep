#[allow(dead_code)]
#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub enum Action {
    Set { key: String, value: String },
    Append { key: String, value: String },
    Delete { key: String },
    Get { key: String },
}

#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub struct Entry {
    pub term: usize,
    pub action: Action,
}
