use std::time::Duration;

use once_cell::sync::Lazy;
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

impl From<ConfigTime> for Duration {
    fn from(time: ConfigTime) -> Self {
        time.to_duration()
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub node_count: usize,
    pub election_timeout: (ConfigTime, ConfigTime),

    pub channel_capacity: usize,
}

impl Config {
    pub fn election_timeout_range(&self) -> (Duration, Duration) {
        (
            self.election_timeout.0.into(),
            self.election_timeout.1.into(),
        )
    }
}

pub static CONFIG: Lazy<Config> =
    Lazy::new(|| ron::from_str(include_str!("../config/config.ron")).expect("Invalid config file"));
