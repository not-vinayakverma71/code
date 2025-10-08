use async_trait::async_trait;
use std::path::{Path, PathBuf};
use anyhow::Result;
use tokio::fs;
use regex::Regex;
use serde_json::{json, Value};

use crate::mcp_tools::{
    core::{Tool, ToolContext, ToolResult, ToolParameter},
    permissions::Permission,
};
pub struct EditFileTool;
impl EditFileTool {
    pub fn new() -> Self {
        Self
    }
}
#[async_trait]
impl Tool for EditFileTool {
    fn name(&self) -> &str {
        "editFile"
    }
    
    fn description(&self) -> &str {
        "Edit file by replacing text"
    }
    
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {"type": "string"},
                "old_str": {"type": "string"},
                "new_str": {"type": "string"}
            },
            "required": ["path", "old_str", "new_str"]
        })
    }
    
    async fn validate(&self, args: &Value) -> Result<()> {
        if !args.is_object() || 
           args.get("path").is_none() || 
           args.get("old_str").is_none() || 
           args.get("new_str").is_none() {
            anyhow::bail!("Missing required parameters");
        }
        Ok(())
    }
    
    async fn execute(&self, args: Value, context: ToolContext) -> Result<ToolResult> {
        let (path_str, search, replace) = if let Some(xml_str) = args.as_str() {
            let tool_use = crate::mcp_tools::xml::parse_tool_use(xml_str)?;
            let path = tool_use.params.get("path")
                .ok_or_else(|| anyhow::anyhow!("Missing path in XML"))?
                .clone();
            let search = tool_use.params.get("search")
                .ok_or_else(|| anyhow::anyhow!("Missing search in XML"))?
                .clone();
            let replace = tool_use.params.get("replace")
                .ok_or_else(|| anyhow::anyhow!("Missing replace in XML"))?
                .clone();
            (path, search, replace)
        } else {
            let path = args.get("path")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("Invalid path parameter"))?
                .to_string();
            let search = args.get("old_str")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("Invalid search parameter"))?
                .to_string();
            let replace = args.get("new_str")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("Invalid replace parameter"))?
                .to_string();
            (path, search, replace)
        };
        
        let path = if Path::new(&path_str).is_absolute() {
            PathBuf::from(&path_str)
        } else {
            context.workspace.join(&path_str)
        };
        
        if !path.exists() {
            let xml_response = format!(
                "<tool_result><success>false</success><error>File not found: {}</error></tool_result>",
                path.display()
            );
            return Ok(ToolResult::from_xml(xml_response));
        }
        let content = fs::read_to_string(&path).await?;
        let occurrences = content.matches(&search).count();
        
        if occurrences == 0 {
            let xml_response = "<tool_result><success>false</success><error>Search pattern not found in file</error></tool_result>".to_string();
            return Ok(ToolResult::from_xml(xml_response));
        }
        
        let new_content = content.replace(&search, &replace);
        fs::write(&path, &new_content).await?;
        
        let xml_response = format!(
            "<tool_result><success>true</success><path>{}</path><occurrences_replaced>{}</occurrences_replaced></tool_result>",
            path.display(),
            occurrences
        );
        Ok(ToolResult::from_xml(xml_response))
    }
    
    fn required_permissions(&self) -> Vec<Permission> {
        vec![Permission::FileWrite("*".to_string())]
    }
}
