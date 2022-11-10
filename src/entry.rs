#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub enum Action {
    Set { key: String, value: String },
    // Get { key: String },
}

#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub struct Entry {
    pub term: usize,
    pub action: Action,
}
