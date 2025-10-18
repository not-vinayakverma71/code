use serde::{Serialize, Deserialize};
use crate::ClineAsk;

// Use ToolProgressStatus from ipc_messages

// ToolParameter moved to types_tool.rs to avoid circular dependencies

#[derive(Debug, Clone)]
pub struct RooTerminalProcess {
    pub id: String,
    pub process: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebviewMessage {
    #[serde(rename = "type")]
    pub msg_type: String,
    pub data: Option<serde_json::Value>,
}
// UserContent already imported from task_exact_translation

// Import Task struct

// Helper functions for ask type checking

fn is_interactive_ask(ask_type: &ClineAsk) -> bool {
    matches!(ask_type, ClineAsk::FollowUp | ClineAsk::Confirmation)
}

fn is_resumable_ask(ask_type: &ClineAsk) -> bool {
    matches!(ask_type, ClineAsk::Tool | ClineAsk::ApiCostLimit)
}
fn is_idle_ask(ask_type: &ClineAsk) -> bool {
    matches!(ask_type, ClineAsk::RequestCostLimit)
}



// RooTerminalProcess placeholder implementation
impl RooTerminalProcess {
    pub fn continue_execution(&self) {
        // Continue terminal execution
    }
    
    pub fn abort(&self) {
        // Abort terminal execution
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_message_routing() {
        // Test message routing
    }
}
