use crate::types_tool::ToolParameter;
use crate::mcp_tools::{core::{Tool, ToolContext, ToolResult, JsonSchema, ResourceLimits}, permissions::Permission};
use async_trait::async_trait;
use serde_json::{json, Value};
use anyhow::Result;
use std::path::PathBuf;
use tokio::fs;

pub struct TaskManager;
impl TaskManager {
    pub fn new() -> Self { Self }
}

pub struct UpdateTodoListTool;
impl UpdateTodoListTool {
    pub fn new() -> Self { Self }
}
#[async_trait]
impl Tool for UpdateTodoListTool {
    fn name(&self) -> &str { "updateTodoList" }
    fn description(&self) -> &str { "Update a todo list with new items, mark items complete, or remove items" }
    fn parameters(&self) -> Vec<crate::mcp_tools::core::ToolParameter> { vec![] }
    fn input_schema(&self) -> Value { 
        json!({
            "type": "object",
            "properties": {
                "file": {
                    "type": "string",
                    "description": "Path to todo file (default: TODO.md)"
                },
                "action": {
                    "type": "string",
                    "enum": ["add", "complete", "remove", "list"],
                    "description": "Action to perform"
                },
                "item": {
                    "type": "string",
                    "description": "Todo item text"
                },
                "index": {
                    "type": "integer",
                    "description": "Item index for complete/remove actions"
                }
            },
            "required": ["action"]
        })
    }
    
    async fn validate(&self, args: &Value) -> Result<()> {
        if !args.is_object() || args.get("action").is_none() {
            anyhow::bail!("Missing required parameter: action");
        }
        Ok(())
    }
    
    async fn execute(&self, args: Value, context: ToolContext) -> Result<ToolResult> {
        let (file_path, action, item, index) = if let Some(xml_str) = args.as_str() {
            let tool_use = crate::mcp_tools::xml::parse_tool_use(xml_str)?;
            let file = tool_use.params.get("file")
                .map(|s| s.clone())
                .unwrap_or_else(|| "TODO.md".to_string());
            let action = tool_use.params.get("action")
                .ok_or_else(|| anyhow::anyhow!("Missing action in XML"))?
                .clone();
            let item = tool_use.params.get("item").map(|s| s.clone());
            let index = tool_use.params.get("index")
                .and_then(|s| s.parse::<usize>().ok());
            (file, action, item, index)
        } else {
            let file = args.get("file")
                .and_then(|v| v.as_str())
                .unwrap_or("TODO.md")
                .to_string();
            let action = args.get("action")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("Invalid action parameter"))?
                .to_string();
            let item = args.get("item")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let index = args.get("index")
                .and_then(|v| v.as_u64())
                .map(|i| i as usize);
            (file, action, item, index)
        };
        
        let path = if file_path.starts_with('/') {
            PathBuf::from(&file_path)
        } else {
            context.workspace.join(&file_path)
        };
        
        // Read existing todos
        let mut todos: Vec<String> = if path.exists() {
            if let Ok(content) = fs::read_to_string(&path).await {
                content.lines()
                    .filter(|line| !line.trim().is_empty())
                    .map(|s| s.to_string())
                    .collect()
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        };
        
        let result = match action.as_str() {
            "add" => {
                if let Some(item) = item {
                    todos.push(format!("- [ ] {}", item));
                    format!("Added: {}", item)
                } else {
                    return Ok(ToolResult::from_xml(
                        "<tool_result><success>false</success><error>Missing item for add action</error></tool_result>".to_string()
                    ));
                }
            }
            "complete" => {
                if let Some(idx) = index {
                    if idx < todos.len() {
                        let old = todos[idx].clone();
                        todos[idx] = todos[idx].replace("- [ ]", "- [x]");
                        format!("Completed: {}", old)
                    } else {
                        return Ok(ToolResult::from_xml(
                            format!("<tool_result><success>false</success><error>Invalid index: {}</error></tool_result>", idx)
                        ));
                    }
                } else {
                    return Ok(ToolResult::from_xml(
                        "<tool_result><success>false</success><error>Missing index for complete action</error></tool_result>".to_string()
                    ));
                }
            }
            "remove" => {
                if let Some(idx) = index {
                    if idx < todos.len() {
                        let removed = todos.remove(idx);
                        format!("Removed: {}", removed)
                    } else {
                        return Ok(ToolResult::from_xml(
                            format!("<tool_result><success>false</success><error>Invalid index: {}</error></tool_result>", idx)
                        ));
                    }
                } else {
                    return Ok(ToolResult::from_xml(
                        "<tool_result><success>false</success><error>Missing index for remove action</error></tool_result>".to_string()
                    ));
                }
            }
            "list" => {
                let mut list = String::new();
                for (i, todo) in todos.iter().enumerate() {
                    list.push_str(&format!("{}: {}\n", i, todo));
                }
                format!("Todo List:\n{}", list)
            }
            _ => {
                return Ok(ToolResult::from_xml(
                    format!("<tool_result><success>false</success><error>Unknown action: {}</error></tool_result>", action)
                ));
            }
        };
        
        // Save updated todos
        if action != "list" {
            let content = todos.join("\n");
            if let Err(e) = fs::write(&path, content).await {
                return Ok(ToolResult::from_xml(
                    format!("<tool_result><success>false</success><error>Failed to save: {}</error></tool_result>", e)
                ));
            }
        }
        
        let xml_response = format!(
            "<tool_result><success>true</success><action>{}</action><result>{}</result><count>{}</count></tool_result>",
            html_escape::encode_text(&action),
            html_escape::encode_text(&result),
            todos.len()
        );
        Ok(ToolResult::from_xml(xml_response))
    }
    
    fn required_permissions(&self) -> Vec<Permission> {
        vec![Permission::FileWrite("*".to_string()), Permission::FileRead("*".to_string())]
    }
}
