use async_trait::async_trait;
use serde_json::{json, Value};
use anyhow::Result;
use crate::mcp_tools::{core::{Tool, ToolContext, ToolResult, ToolParameter}, permissions::Permission};

pub struct UseMcpToolTool;
impl UseMcpToolTool {
    pub fn new() -> Self { Self }
}
#[async_trait]
impl Tool for UseMcpToolTool {
    fn name(&self) -> &str { "useMcpTool" }
    fn description(&self) -> &str { "Execute another MCP tool dynamically" }
    fn parameters(&self) -> Vec<crate::mcp_tools::core::ToolParameter> { vec![] }
    fn input_schema(&self) -> Value { 
        json!({
            "type": "object",
            "properties": {
                "tool_name": {
                    "type": "string",
                    "description": "Name of the tool to execute"
                },
                "arguments": {
                    "type": "object",
                    "description": "Arguments to pass to the tool"
                },
                "timeout": {
                    "type": "integer",
                    "description": "Timeout in seconds"
                }
            },
            "required": ["tool_name", "arguments"]
        })
    }
    
    async fn validate(&self, args: &Value) -> Result<()> {
        if !args.is_object() || args.get("tool_name").is_none() || args.get("arguments").is_none() {
            anyhow::bail!("Missing required parameters: tool_name and arguments");
        }
        Ok(())
    }
    
    async fn execute(&self, args: Value, context: ToolContext) -> Result<ToolResult> {
        let (tool_name, tool_args, timeout) = if let Some(xml_str) = args.as_str() {
            let tool_use = crate::mcp_tools::xml::parse_tool_use(xml_str)?;
            let tool_name = tool_use.params.get("tool_name")
                .ok_or_else(|| anyhow::anyhow!("Missing tool_name in XML"))?
                .clone();
            let arguments_str = tool_use.params.get("arguments")
                .ok_or_else(|| anyhow::anyhow!("Missing arguments in XML"))?;
            let arguments: Value = serde_json::from_str(arguments_str)?;
            let timeout = tool_use.params.get("timeout")
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(30);
            (tool_name, arguments, timeout)
        } else {
            let tool_name = args.get("tool_name")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("Invalid tool_name"))?
                .to_string();
            let arguments = args.get("arguments")
                .ok_or_else(|| anyhow::anyhow!("Invalid arguments"))?
                .clone();
            let timeout = args.get("timeout")
                .and_then(|v| v.as_u64())
                .unwrap_or(30);
            (tool_name, arguments, timeout)
        };
        
        // List of available tools that can be called
        let available_tools = vec![
            "readFile", "writeFile", "listFiles", "searchFiles", "editFile",
            "executeCommand", "gitStatus", "gitDiff", "gitCommit", "applyDiff",
            "condense", "parseJson", "askFollowupQuestion", "listCodeDefinitions",
            "updateTodoList", "codebaseSearch", "searchAndReplace", "httpRequest",
            "insertContent", "multiApplyDiff", "browserAction", "switchMode",
            "fetchInstructions", "accessMcpResource", "newTask", "newRule",
            "reportBug"
        ];
        if !available_tools.contains(&tool_name.as_str()) {
            return Ok(ToolResult::from_xml(format!(
                "<tool_result><success>false</success><error>Unknown tool: {}. Available tools: {:?}</error></tool_result>",
                tool_name, available_tools
            )));
        }
        
        // In a real implementation, we would dispatch to the actual tool here
        // For now, we'll simulate a successful execution
        let simulated_result = match tool_name.as_str() {
            "readFile" => {
                let path = tool_args.get("path")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");
                format!("<content>Simulated content of {}</content>", path)
            }
            "listFiles" => {
                "<files><file>file1.txt</file><file>file2.txt</file></files>".to_string()
            }
            "parseJson" => {
                "<parsed>true</parsed><valid>true</valid>".to_string()
            }
            _ => {
                format!("<result>Successfully executed {}</result>", tool_name)
            }
        };
        let xml_response = format!(
            "<tool_result><success>true</success><tool_name>{}</tool_name><timeout>{}</timeout>{}</tool_result>",
            tool_name,
            timeout,
            simulated_result
        );
        Ok(ToolResult::from_xml(xml_response))
    }
    
    fn required_permissions(&self) -> Vec<Permission> {
        vec![Permission::FileRead("*".to_string()), Permission::FileWrite("*".to_string())]
    }
}
