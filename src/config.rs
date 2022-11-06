use std::time::Duration;

use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone, Copy)]
pub enum ConfigTime {
    Second(u64),
    Millisecond(u64),
    Microsecond(u64),
    Nanosecond(u64),
    Minute(u64),
}

impl ConfigTime {
    fn to_duration(self) -> Duration {
        match self {
            Self::Second(n) => Duration::from_secs(n),
            Self::Millisecond(n) => Duration::from_millis(n),
            Self::Microsecond(n) => Duration::from_micros(n),
            Self::Nanosecond(n) => Duration::from_nanos(n),
            Self::Minute(n) => Duration::from_secs(n * 60),
        }
    }
}

impl Into<Duration> for ConfigTime {
    fn into(self) -> Duration {
        self.to_duration()
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub node_count: usize,
    pub election_timeout: (ConfigTime, ConfigTime),
}
