pub type NodeId = usize;

pub struct Node {
    pub recv:Receiver<Message>,
    pub senders:  Vec<Sender<Messag
}
