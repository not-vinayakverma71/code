// Retry handler for MCP tools - clean implementation
use std::time::Duration;

pub struct RetryHandler {
    max_retries: u32,
    base_delay: Duration,
}

impl RetryHandler {
    pub fn new() -> Self {
        Self {
            max_retries: 3,
            base_delay: Duration::from_millis(100),
        }
    }
}
