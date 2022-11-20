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
