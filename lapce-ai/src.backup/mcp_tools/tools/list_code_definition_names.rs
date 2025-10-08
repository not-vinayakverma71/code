use async_trait::async_trait;
use serde_json::{json, Value};
use anyhow::Result;
use crate::mcp_tools::{
    core::{Tool, ToolContext, ToolResult, ToolParameter, ResourceLimits}, 
    permissions::Permission
};

pub struct ListCodeDefinitionNamesTool;

impl ListCodeDefinitionNamesTool {
    pub fn new() -> Self { Self }
}

#[async_trait]
impl Tool for ListCodeDefinitionNamesTool {
    fn name(&self) -> &str { "listCodeDefinitionNames" }
    fn description(&self) -> &str { "List names of code definitions" }
    fn parameters(&self) -> Vec<ToolParameter> { vec![] }
    fn input_schema(&self) -> Value { 
        json!({
            "type": "object",
            "properties": {
                "path": {"type": "string"}
            },
            "required": ["path"]
        })
    }
    
    async fn validate(&self, args: &Value) -> Result<()> { Ok(()) }
    
    async fn execute(&self, args: Value, context: ToolContext) -> Result<ToolResult> {
        Ok(ToolResult::success(json!({
            "definitions": ["fn main", "struct Config", "impl Tool"]
        })))
    }
    
    fn required_permissions(&self) -> Vec<Permission> { 
        vec![Permission::FileRead("*".to_string())]
    }
    
    fn resource_limits(&self) -> ResourceLimits {
        ResourceLimits::default()
    }
}
