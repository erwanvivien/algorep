use std::{pin::Pin, time::Duration};

use tokio::time::Sleep;

use super::Node;

impl Node {
    /// Helper function to generate duration for a timeout
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
