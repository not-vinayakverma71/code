use async_trait::async_trait;
use serde_json::{json, Value};
use anyhow::Result;
use crate::types_tool::ToolParameter;

pub struct ConversationManager;
impl ConversationManager {
    pub fn new() -> Self { Self }
}

use crate::mcp_tools::{
    core::{Tool, ToolContext, ToolResult, JsonSchema, ResourceLimits},
    permissions::Permission,
};

pub struct CondenseTool;
impl CondenseTool { pub fn new() -> Self { Self } }
#[async_trait]
impl Tool for CondenseTool {
    fn name(&self) -> &str { "condense" }
    fn description(&self) -> &str { "Condense text" }
    fn parameters(&self) -> Vec<crate::mcp_tools::core::ToolParameter> { vec![] }
    fn input_schema(&self) -> Value { 
        json!({"type": "object", "properties": {"content": {"type": "string"}, "max_length": {"type": "integer"}}})
    }
    async fn validate(&self, _: &Value) -> Result<()> { Ok(()) }
    async fn execute(&self, args: Value, _context: ToolContext) -> Result<ToolResult> {
        let (text, max_length) = if let Some(xml_str) = args.as_str() {
            let tool_use = crate::mcp_tools::xml::parse_tool_use(xml_str)?;
            let text = tool_use.params.get("text")
                .ok_or_else(|| anyhow::anyhow!("Missing text in XML"))?
                .clone();
            let max_length = tool_use.params.get("max_length")
                .and_then(|s| s.parse::<usize>().ok())
                .unwrap_or(100);
            (text, max_length)
        } else {
            let text = args.get("text")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("Invalid text parameter"))?
                .to_string();
            let max_length = args.get("max_length")
                .and_then(|v| v.as_u64())
                .unwrap_or(100) as usize;
            (text, max_length)
        };
        
        // Remove extra whitespace
        let condensed = text
            .lines()
            .map(|line| line.trim())
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>()
            .join(" ");
        // Truncate if needed
        let result = if condensed.len() > max_length {
            format!("{}...", &condensed[..max_length - 3]) // subtract 3 to account for the ellipsis
        } else {
            condensed
        };
        let xml_response = format!(
            "<tool_result><success>true</success><original_length>{}</original_length><condensed_length>{}</condensed_length><text>{}</text></tool_result>",
            text.len(),
            result.len(),
            html_escape::encode_text(&result)
        );
        Ok(ToolResult::from_xml(xml_response))
    }
    
    fn required_permissions(&self) -> Vec<Permission> {
        vec![Permission::FileRead("*".to_string())]
    }
}
