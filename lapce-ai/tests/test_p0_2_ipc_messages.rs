// P0-2 Tests: IPC Messages Serialization
// Tests the new tool execution lifecycle messages

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};
    use std::path::PathBuf;
    
    // Copy the message structures here to test them independently
    
    #[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
    #[serde(rename_all = "lowercase")]
    pub enum IpcOrigin {
        Client,
        Server,
    }
    
    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub enum ToolExecutionStatus {
        Started {
            execution_id: String,
            tool_name: String,
            timestamp: u64,
        },
        Progress {
            execution_id: String,
            message: String,
            percentage: Option<u8>,
        },
        Completed {
            execution_id: String,
            result: serde_json::Value,
            duration_ms: u64,
        },
        Failed {
            execution_id: String,
            error: String,
            duration_ms: u64,
        },
    }
    
    #[derive(Debug, Clone, Copy, Serialize, Deserialize)]
    #[serde(rename_all = "lowercase")]
    pub enum StreamType {
        Stdout,
        Stderr,
    }
    
    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub enum CommandExecutionStatusMessage {
        Started {
            execution_id: String,
            command: String,
            args: Vec<String>,
            cwd: Option<PathBuf>,
        },
        Output {
            execution_id: String,
            stream_type: StreamType,
            line: String,
            timestamp: u64,
        },
        Completed {
            execution_id: String,
            exit_code: i32,
            duration_ms: u64,
        },
        Timeout {
            execution_id: String,
            duration_ms: u64,
        },
    }
    
    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub enum DiffOperationMessage {
        OpenDiffFiles {
            left_path: PathBuf,
            right_path: PathBuf,
            title: Option<String>,
        },
        DiffSave {
            file_path: PathBuf,
            content: String,
        },
        DiffRevert {
            file_path: PathBuf,
        },
        CloseDiff {
            left_path: PathBuf,
            right_path: PathBuf,
        },
    }
    
    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct ToolApprovalRequest {
        pub execution_id: String,
        pub tool_name: String,
        pub operation: String,
        pub target: String,
        pub details: String,
        pub require_confirmation: bool,
    }
    
    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct ToolApprovalResponse {
        pub execution_id: String,
        pub approved: bool,
        pub reason: Option<String>,
    }
    
    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(tag = "type")]
    pub enum ToolIpcMessage {
        #[serde(rename = "ToolExecutionStatus")]
        ToolExecutionStatus {
            origin: IpcOrigin,
            data: ToolExecutionStatus,
        },
        #[serde(rename = "CommandExecutionStatus")]
        CommandExecutionStatus {
            origin: IpcOrigin,
            data: CommandExecutionStatusMessage,
        },
        #[serde(rename = "DiffOperation")]
        DiffOperation {
            origin: IpcOrigin,
            data: DiffOperationMessage,
        },
        #[serde(rename = "ToolApprovalRequest")]
        ToolApprovalRequest {
            origin: IpcOrigin,
            data: ToolApprovalRequest,
        },
        #[serde(rename = "ToolApprovalResponse")]
        ToolApprovalResponse {
            origin: IpcOrigin,
            data: ToolApprovalResponse,
        },
    }
    
    // Now the actual tests
    
    #[test]
    fn test_tool_execution_status_serialization() {
        let status = ToolExecutionStatus::Started {
            execution_id: "test-123".to_string(),
            tool_name: "readFile".to_string(),
            timestamp: 1234567890,
        };
        
        let json = serde_json::to_string(&status).unwrap();
        println!("Serialized: {}", json);
        
        let deserialized: ToolExecutionStatus = serde_json::from_str(&json).unwrap();
        
        match deserialized {
            ToolExecutionStatus::Started { execution_id, tool_name, timestamp } => {
                assert_eq!(execution_id, "test-123");
                assert_eq!(tool_name, "readFile");
                assert_eq!(timestamp, 1234567890);
            }
            _ => panic!("Wrong variant"),
        }
    }
    
    #[test]
    fn test_command_execution_status_serialization() {
        let status = CommandExecutionStatusMessage::Output {
            execution_id: "cmd-456".to_string(),
            stream_type: StreamType::Stdout,
            line: "Hello, world!".to_string(),
            timestamp: 9876543210,
        };
        
        let json = serde_json::to_string(&status).unwrap();
        println!("Serialized: {}", json);
        
        let deserialized: CommandExecutionStatusMessage = serde_json::from_str(&json).unwrap();
        
        match deserialized {
            CommandExecutionStatusMessage::Output { execution_id, stream_type, line, timestamp } => {
                assert_eq!(execution_id, "cmd-456");
                assert!(matches!(stream_type, StreamType::Stdout));
                assert_eq!(line, "Hello, world!");
                assert_eq!(timestamp, 9876543210);
            }
            _ => panic!("Wrong variant"),
        }
    }
    
    #[test]
    fn test_diff_operation_message_serialization() {
        let msg = DiffOperationMessage::OpenDiffFiles {
            left_path: PathBuf::from("/path/to/original.txt"),
            right_path: PathBuf::from("/path/to/modified.txt"),
            title: Some("Test Diff".to_string()),
        };
        
        let json = serde_json::to_string(&msg).unwrap();
        println!("Serialized: {}", json);
        
        let deserialized: DiffOperationMessage = serde_json::from_str(&json).unwrap();
        
        match deserialized {
            DiffOperationMessage::OpenDiffFiles { left_path, right_path, title } => {
                assert_eq!(left_path, PathBuf::from("/path/to/original.txt"));
                assert_eq!(right_path, PathBuf::from("/path/to/modified.txt"));
                assert_eq!(title, Some("Test Diff".to_string()));
            }
            _ => panic!("Wrong variant"),
        }
    }
    
    #[test]
    fn test_tool_approval_request_serialization() {
        let request = ToolApprovalRequest {
            execution_id: "exec-789".to_string(),
            tool_name: "writeFile".to_string(),
            operation: "write".to_string(),
            target: "/important/file.txt".to_string(),
            details: "Writing sensitive data".to_string(),
            require_confirmation: true,
        };
        
        let json = serde_json::to_string(&request).unwrap();
        println!("Serialized: {}", json);
        assert!(json.contains("\"executionId\":\"exec-789\""));
        
        let deserialized: ToolApprovalRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.execution_id, "exec-789");
        assert_eq!(deserialized.tool_name, "writeFile");
        assert!(deserialized.require_confirmation);
    }
    
    #[test]
    fn test_tool_approval_response_serialization() {
        let response = ToolApprovalResponse {
            execution_id: "exec-789".to_string(),
            approved: false,
            reason: Some("User rejected the operation".to_string()),
        };
        
        let json = serde_json::to_string(&response).unwrap();
        println!("Serialized: {}", json);
        
        let deserialized: ToolApprovalResponse = serde_json::from_str(&json).unwrap();
        
        assert_eq!(deserialized.execution_id, "exec-789");
        assert!(!deserialized.approved);
        assert_eq!(deserialized.reason, Some("User rejected the operation".to_string()));
    }
    
    #[test]
    fn test_tool_ipc_message_serialization() {
        let msg = ToolIpcMessage::ToolExecutionStatus {
            origin: IpcOrigin::Server,
            data: ToolExecutionStatus::Completed {
                execution_id: "test-complete".to_string(),
                result: serde_json::json!({"success": true}),
                duration_ms: 150,
            },
        };
        
        let json = serde_json::to_string(&msg).unwrap();
        println!("Serialized: {}", json);
        assert!(json.contains("\"type\":\"ToolExecutionStatus\""));
        assert!(json.contains("\"origin\":\"server\""));
        
        let deserialized: ToolIpcMessage = serde_json::from_str(&json).unwrap();
        match deserialized {
            ToolIpcMessage::ToolExecutionStatus { origin, data } => {
                assert_eq!(origin, IpcOrigin::Server);
                match data {
                    ToolExecutionStatus::Completed { duration_ms, .. } => {
                        assert_eq!(duration_ms, 150);
                    }
                    _ => panic!("Wrong status variant"),
                }
            }
            _ => panic!("Wrong message variant"),
        }
    }
    
    #[test]
    fn test_stream_type_serialization() {
        assert_eq!(serde_json::to_string(&StreamType::Stdout).unwrap(), "\"stdout\"");
        assert_eq!(serde_json::to_string(&StreamType::Stderr).unwrap(), "\"stderr\"");
        
        let stdout: StreamType = serde_json::from_str("\"stdout\"").unwrap();
        let stderr: StreamType = serde_json::from_str("\"stderr\"").unwrap();
        
        assert!(matches!(stdout, StreamType::Stdout));
        assert!(matches!(stderr, StreamType::Stderr));
    }
    
    #[test]
    fn test_all_message_variants() {
        // Test all ToolExecutionStatus variants
        let variants = vec![
            ToolExecutionStatus::Started {
                execution_id: "1".to_string(),
                tool_name: "test".to_string(),
                timestamp: 1000,
            },
            ToolExecutionStatus::Progress {
                execution_id: "2".to_string(),
                message: "Processing...".to_string(),
                percentage: Some(50),
            },
            ToolExecutionStatus::Completed {
                execution_id: "3".to_string(),
                result: serde_json::json!({"done": true}),
                duration_ms: 100,
            },
            ToolExecutionStatus::Failed {
                execution_id: "4".to_string(),
                error: "Error occurred".to_string(),
                duration_ms: 50,
            },
        ];
        
        for variant in variants {
            let json = serde_json::to_string(&variant).unwrap();
            let _deserialized: ToolExecutionStatus = serde_json::from_str(&json).unwrap();
        }
        
        println!("✅ All ToolExecutionStatus variants serialize/deserialize correctly");
    }
    
    #[test]
    fn test_json_field_names() {
        // Test that camelCase is correctly applied
        let request = ToolApprovalRequest {
            execution_id: "test".to_string(),
            tool_name: "tool".to_string(),
            operation: "op".to_string(),
            target: "target".to_string(),
            details: "details".to_string(),
            require_confirmation: true,
        };
        
        let json = serde_json::to_string(&request).unwrap();
        
        // Check for camelCase field names
        assert!(json.contains("\"executionId\""));
        assert!(json.contains("\"toolName\""));
        assert!(json.contains("\"requireConfirmation\""));
        assert!(!json.contains("\"execution_id\""));
        assert!(!json.contains("\"tool_name\""));
        
        println!("✅ JSON field names are correctly in camelCase");
    }
}
