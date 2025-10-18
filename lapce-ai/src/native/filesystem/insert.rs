use std::path::Path;
use crate::mcp_tools::{
    core::{Tool, ToolContext, ToolResult, ToolParameter, ResourceLimits},
    permissions::Permission,
};
use async_trait::async_trait;
use anyhow::Result;
use serde_json::{json, Value};
use tokio::fs;
use std::path::PathBuf;

pub struct InsertContentTool;
impl InsertContentTool {
    pub fn new() -> Self { Self }
}
#[async_trait]
impl Tool for InsertContentTool {
    fn name(&self) -> &str { "insertContent" }
    fn description(&self) -> &str { "Insert content at specific position in file" }
    fn parameters(&self) -> Vec<ToolParameter> { vec![] }
    fn input_schema(&self) -> Value { 
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "File path"
                },
                "content": {
                    "type": "string",
                    "description": "Content to insert"
                },
                "position": {
                    "type": "string",
                    "enum": ["start", "end", "line", "after", "before"],
                    "description": "Where to insert"
                },
                "line_number": {
                    "type": "integer",
                    "description": "Line number for line-based insertion"
                },
                "pattern": {
                    "type": "string",
                    "description": "Pattern to search for before/after insertion"
                }
            },
            "required": ["path", "content", "position"]
        })
    }
    
    async fn validate(&self, args: &Value) -> Result<()> {
        if !args.is_object() || args.get("path").is_none() || 
           args.get("content").is_none() || args.get("position").is_none() {
            anyhow::bail!("Missing required parameters");
        }
        Ok(())
    }
    
    async fn execute(&self, args: Value, context: ToolContext) -> Result<ToolResult> {
        let (path_str, content, position, line_number, pattern) = if let Some(xml_str) = args.as_str() {
            let tool_use = crate::mcp_tools::xml::parse_tool_use(xml_str)?;
            let path = tool_use.params.get("path")
                .ok_or_else(|| anyhow::anyhow!("Missing path in XML"))?
                .clone();
            let content = tool_use.params.get("content")
                .ok_or_else(|| anyhow::anyhow!("Missing content in XML"))?
                .clone();
            let position = tool_use.params.get("position")
                .ok_or_else(|| anyhow::anyhow!("Missing position in XML"))?
                .clone();
            let line_number = tool_use.params.get("line_number")
                .and_then(|s| s.parse::<usize>().ok());
            let pattern = tool_use.params.get("pattern").map(|s| s.clone());
            (path, content, position, line_number, pattern)
        } else {
            let path = args.get("path")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("Invalid path"))?
                .to_string();
            let content = args.get("content")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("Invalid content"))?
                .to_string();
            let position = args.get("position")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("Invalid position"))?
                .to_string();
            let line_number = args.get("line_number")
                .and_then(|v| v.as_u64())
                .map(|n| n as usize);
            let pattern = args.get("pattern")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            (path, content, position, line_number, pattern)
        };
        
        let path = if Path::new(&path_str).is_absolute() {
            PathBuf::from(&path_str)
        } else {
            context.workspace.join(&path_str)
        };
        
        let existing = if path.exists() {
            fs::read_to_string(&path).await?
        } else {
            String::new()
        };
        
        let new_content = match position.as_str() {
            "start" => format!("{}\n{}", content, existing),
            "end" => format!("{}\n{}", existing, content),
            "line" => {
                let line_num = line_number.ok_or_else(|| anyhow::anyhow!("Missing line_number"))?;
                let lines: Vec<&str> = existing.lines().collect();
                let mut result = String::new();
                for (i, line) in lines.iter().enumerate() {
                    if i == line_num - 1 {
                        result.push_str(&content);
                        result.push('\n');
                    }
                    result.push_str(line);
                    result.push('\n');
                }
                result
            }
            "before" => {
                let pat = pattern.ok_or_else(|| anyhow::anyhow!("Missing pattern for before insertion"))?;
                if let Some(pos) = existing.find(&pat) {
                    let mut result = String::new();
                    result.push_str(&existing[..pos]);
                    result.push_str(&content);
                    result.push_str(&existing[pos..]);
                    result
                } else {
                    return Ok(ToolResult::from_xml(format!(
                        "<tool_result><success>false</success><error>Pattern not found: {}</error></tool_result>",
                        pat
                    )));
                }
            }
            "after" => {
                let pat = pattern.ok_or_else(|| anyhow::anyhow!("Missing pattern for after insertion"))?;
                if let Some(pos) = existing.find(&pat) {
                    let mut result = String::new();
                    let end_pos = pos + pat.len();
                    result.push_str(&existing[..end_pos]);
                    result.push_str(&content);
                    result.push_str(&existing[end_pos..]);
                    result
                } else {
                    return Ok(ToolResult::from_xml(format!(
                        "<tool_result><success>false</success><error>Pattern not found: {}</error></tool_result>",
                        pat
                    )));
                }
            }
            _ => {
                return Ok(ToolResult::from_xml(format!(
                    "<tool_result><success>false</success><error>Invalid position: {}</error></tool_result>",
                    position
                )));
            }
        };
        
        fs::write(&path, new_content).await?;
        let xml_response = format!(
            "<tool_result><success>true</success><path>{}</path><position>{}</position></tool_result>",
            path.display(),
            position
        );
        Ok(ToolResult::from_xml(xml_response))
    }
    
    fn required_permissions(&self) -> Vec<Permission> {
        vec![Permission::FileWrite("*".to_string())]
    }
}
