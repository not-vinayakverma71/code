/// Tool Execution Timeout Handler
use std::time::Duration;
use tokio::time::timeout;
use anyhow::{Result, bail};

pub struct TimeoutHandler {
    default_timeout: Duration,
    tool_timeouts: std::collections::HashMap<String, Duration>,
}

impl TimeoutHandler {
    pub fn new() -> Self {
        let mut tool_timeouts = std::collections::HashMap::new();
        
        // Set specific timeouts for different tools
        tool_timeouts.insert("readFile".to_string(), Duration::from_secs(5));
        tool_timeouts.insert("writeFile".to_string(), Duration::from_secs(10));
        tool_timeouts.insert("executeCommand".to_string(), Duration::from_secs(30));
        tool_timeouts.insert("searchFiles".to_string(), Duration::from_secs(60));
        tool_timeouts.insert("codebaseSearch".to_string(), Duration::from_secs(120));
        
        Self {
            default_timeout: Duration::from_secs(30),
            tool_timeouts,
        }
    }
    
    pub async fn execute_with_timeout<F, T>(
        &self,
        tool_name: &str,
        operation: F,
    ) -> Result<T>
    where
        F: std::future::Future<Output = Result<T>>,
    {
        let timeout_duration = self.tool_timeouts
            .get(tool_name)
            .copied()
            .unwrap_or(self.default_timeout);
        
        match timeout(timeout_duration, operation).await {
            Ok(result) => result,
            Err(_) => bail!(
                "Tool '{}' execution timed out after {} seconds",
                tool_name,
                timeout_duration.as_secs()
            ),
        }
    }
    
    pub fn get_timeout(&self, tool_name: &str) -> Duration {
        self.tool_timeouts
            .get(tool_name)
            .copied()
            .unwrap_or(self.default_timeout)
    }
}
