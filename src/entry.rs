#[allow(dead_code)]
#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub enum StateMutation {
    Create { uid: String, filename: String },
    Delete { uid: String },
    Append { uid: String, text: String },
}

#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub struct Entry {
    pub term: usize,
    pub action: StateMutation,
}
