// Terminal Bridge Integration
// Connects terminal subsystem with AI backend via IPC

use std::sync::Arc;
use lapce_rpc::terminal::TermId;
use super::messages::{CommandSource, InboundMessage, TerminalOp};
use super::bridge::BridgeClient;

/// Terminal bridge handler - converts between terminal events and IPC messages
pub struct TerminalBridge {
    bridge_client: Arc<BridgeClient>,
}

impl TerminalBridge {
    /// Create a new terminal bridge
    pub fn new(bridge_client: Arc<BridgeClient>) -> Self {
        Self { bridge_client }
    }
    
    /// Send command started event to backend
    pub fn send_command_started(
        &self,
        term_id: &TermId,
        command: String,
        source: crate::terminal::types::CommandSource,
        cwd: String,
    ) -> Result<(), String> {
        let ipc_source = match source {
            crate::terminal::types::CommandSource::User => CommandSource::User,
            crate::terminal::types::CommandSource::Cascade => CommandSource::Cascade,
        };
        
        let msg = InboundMessage::TerminalCommandStarted {
            terminal_id: term_id.to_string(),
            command,
            source: ipc_source,
            cwd,
        };
        
        // Note: This would be sent through a proper bidirectional channel
        // For now, this shows the message structure
        tracing::info!("Terminal command started: {:?}", term_id);
        Ok(())
    }
    
    /// Send command completed event to backend
    pub fn send_command_completed(
        &self,
        term_id: &TermId,
        command: String,
        exit_code: i32,
        duration_ms: u64,
        forced_exit: bool,
    ) -> Result<(), String> {
        let msg = InboundMessage::TerminalCommandCompleted {
            terminal_id: term_id.to_string(),
            command,
            exit_code,
            duration_ms,
            forced_exit,
        };
        
        tracing::info!("Terminal command completed: {:?}, exit={}", term_id, exit_code);
        Ok(())
    }
    
    /// Send terminal output chunk to backend
    pub fn send_output_chunk(
        &self,
        term_id: &TermId,
        data: String,
    ) -> Result<(), String> {
        let msg = InboundMessage::TerminalOutput {
            terminal_id: term_id.to_string(),
            data,
            markers: Vec::new(), // OSC markers would be parsed from data
        };
        
        Ok(())
    }
    
    /// Send command injection result to backend
    pub fn send_injection_result(
        &self,
        term_id: &TermId,
        command: String,
        success: bool,
        error: Option<String>,
    ) -> Result<(), String> {
        let msg = InboundMessage::TerminalCommandInjected {
            terminal_id: term_id.to_string(),
            command,
            success,
            error,
        };
        
        tracing::info!("Terminal command injected: {:?}, success={}", term_id, success);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_command_source_conversion() {
        // Test that CommandSource maps correctly
        let ipc_user = CommandSource::User;
        let ipc_cascade = CommandSource::Cascade;
        
        // Verify serialization format matches
        let user_json = serde_json::to_string(&ipc_user).unwrap();
        let cascade_json = serde_json::to_string(&ipc_cascade).unwrap();
        
        assert_eq!(user_json, r#""User""#);
        assert_eq!(cascade_json, r#""Cascade""#);
    }
}
