use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
struct Config {
    servers: u16,
    election_timeout: (f32, f32),
}

const CONFIG_STR: &'static str = include_str!("../config/config.ron");

fn main() {
    let config: Config = ron::from_str(CONFIG_STR).expect("");
}
