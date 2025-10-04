use crate::types_tool::ToolParameter;

pub struct InteractionHandler;
impl InteractionHandler {
    pub fn new() -> Self { Self }
}
use crate::mcp_tools::{core::{Tool, ToolContext, ToolResult, JsonSchema, ResourceLimits}, permissions::Permission};
use async_trait::async_trait;
use serde_json::{json, Value};
use anyhow::Result;

pub struct AskFollowupQuestionTool;
impl AskFollowupQuestionTool {
    pub fn new() -> Self { Self }
}
#[async_trait]
impl Tool for AskFollowupQuestionTool {
    fn name(&self) -> &str { "askFollowupQuestion" }
    fn description(&self) -> &str { "Ask a followup question to clarify task requirements" }
    fn parameters(&self) -> Vec<crate::mcp_tools::core::ToolParameter> { vec![] }
    fn input_schema(&self) -> Value { 
        json!({
            "type": "object",
            "properties": {
                "question": {
                    "type": "string",
                    "description": "The followup question to ask"
                },
                "context": {
                    "type": "string", 
                    "description": "Context about why this question is needed"
                }
            },
            "required": ["question"]
        })
    }
    
    async fn validate(&self, args: &Value) -> Result<()> {
        if !args.is_object() || args.get("question").is_none() {
            anyhow::bail!("Missing required parameter: question");
        }
        Ok(())
    }
    
    async fn execute(&self, args: Value, _context: ToolContext) -> Result<ToolResult> {
        let (question, context) = if let Some(xml_str) = args.as_str() {
            let tool_use = crate::mcp_tools::xml::parse_tool_use(xml_str)?;
            let question = tool_use.params.get("question")
                .ok_or_else(|| anyhow::anyhow!("Missing question in XML"))?
                .clone();
            let context = tool_use.params.get("context")
                .map(|s| s.clone())
                .unwrap_or_default();
            (question, context)
        } else {
            let question = args.get("question")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("Invalid question parameter"))?
                .to_string();
            let context = args.get("context")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            (question, context)
        };
        
        // Generate followup response
        let response = if !context.is_empty() {
            format!("Followup Question: {}\nContext: {}", question, context)
        } else {
            format!("Followup Question: {}", question)
        };
        
        let xml_response = format!(
            "<tool_result><success>true</success><question>{}</question><response>{}</response></tool_result>",
            html_escape::encode_text(&question),
            html_escape::encode_text(&response)
        );
        Ok(ToolResult::from_xml(xml_response))
    }
    
    fn required_permissions(&self) -> Vec<Permission> { vec![] }
}
