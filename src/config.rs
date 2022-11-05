use std::time::Duration;

use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub enum ConfigTime {
    Second(u64),
    Millisecond(u64),
    Microsecond(u64),
    Nanosecond(u64),
    Minute(u64),
}

impl ConfigTime {
    #[allow(dead_code)]
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

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub servers: usize,
    pub election_timeout: (ConfigTime, ConfigTime),
}
