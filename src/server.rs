use std::thread::JoinHandle;

pub type ServerId = usize;

pub struct Server {
    pub thread: JoinHandle<()>,
    pub id: ServerId,
}
