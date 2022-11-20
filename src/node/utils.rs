use std::{cmp::min, collections::VecDeque, fs::OpenOptions, pin::Pin, time::Duration};

use log::{debug, error, info};
use serde::{Deserialize, Serialize};
use tokio::{
    sync::mpsc::{Receiver, Sender},
    time::Sleep,
};

use crate::{
    entry::{LogEntry, StateMutation},
    message::{
        ClientCommand, ClientResponse, ClientResponseError, Message, MessageContent, ReplAction,
    },
    role::{CandidateData, LeaderData, Role, Waiter},
    state::VolatileState,
    CONFIG,
};

use super::Node;

impl Node {
    pub(super) fn generate_timeout(&self) -> Pin<Box<Sleep>> {
        let duration = {
            let (min, max) = self.election_timeout_range;

            if self.role.is_leader() {
                min / 2
            } else if max == min {
                min
            } else {
                let timeout =
                    rand::random::<u128>() % (max.as_millis() - min.as_millis()) + min.as_millis();
                Duration::from_millis(timeout as u64)
            }
        };

        Box::pin(tokio::time::sleep(duration))
    }
}
