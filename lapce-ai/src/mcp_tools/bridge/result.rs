// Result conversion: core::tools::ToolOutput → mcp_tools::core::ToolResult

use crate::core::tools::traits::ToolOutput;
use crate::mcp_tools::core::ToolResult;

/// Convert core tool output to MCP tool result
pub fn core_output_to_mcp(output: ToolOutput) -> ToolResult {
    ToolResult {
        success: output.success,
        data: Some(output.result),
        error: output.error,
        metadata: if output.metadata.is_empty() { None } else { Some(serde_json::to_value(output.metadata).unwrap_or(serde_json::Value::Null)) },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    
    #[test]
    fn test_success_conversion() {
        let core_output = ToolOutput::success(json!({
            "content": "Hello World",
            "path": "test.txt"
        }));
        
        let mcp_result = core_output_to_mcp(core_output);
        
        assert!(mcp_result.success);
        assert_eq!(mcp_result.data.unwrap()["content"], "Hello World");
        assert!(mcp_result.error.is_none());
    }
    
    #[test]
    fn test_error_conversion() {
        let core_output = ToolOutput {
            success: false,
            result: json!({}),
            error: Some("File not found".to_string()),
            metadata: Default::default(),
        };
        
        let mcp_result = core_output_to_mcp(core_output);
        
        assert!(!mcp_result.success);
        assert_eq!(mcp_result.error.unwrap(), "File not found");
    }
    
    #[test]
    fn test_metadata_preserved() {
        let core_output = ToolOutput {
            success: true,
            result: json!({"data": "test"}),
            error: None,
            metadata: json!({"duration_ms": 42, "cached": true}),
        };
        
        let mcp_result = core_output_to_mcp(core_output);
        
        assert!(mcp_result.success);
        assert_eq!(mcp_result.metadata.unwrap()["duration_ms"], 42);
        assert_eq!(mcp_result.metadata.unwrap()["cached"], true);
    }
}
