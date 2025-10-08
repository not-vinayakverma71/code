/// MCP Types - EXACT 1:1 Translation from TypeScript
/// Source: Codex/packages/types/src/mcp.ts
use serde::{Deserialize, Serialize};

/// McpExecutionStatus - Direct translation from TypeScript discriminated union
/// Lines 7-31 from mcp.ts
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status")]
pub enum McpExecutionStatus {
    #[serde(rename = "started")]
    Started {
        #[serde(rename = "executionId")]
        execution_id: String,
        #[serde(rename = "serverName")]
        server_name: String,
        #[serde(rename = "toolName")]
        tool_name: String,
    },
    #[serde(rename = "output")]
    Output {
        #[serde(rename = "executionId")]
        execution_id: String,
        response: String,
    },
    #[serde(rename = "completed")]
    Completed {
        #[serde(rename = "executionId")]
        execution_id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        response: Option<String>,
    },
    #[serde(rename = "error")]
    Error {
        #[serde(rename = "executionId")]
        execution_id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<String>,
    },
}
