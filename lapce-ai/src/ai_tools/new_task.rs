use crate::types_tool::ToolParameter;
use crate::mcp_tools::{
    core::{Tool, ToolContext, ToolResult, JsonSchema, ResourceLimits},
    permissions::Permission,
};
use async_trait::async_trait;
use serde_json::{json, Value};
use anyhow::Result;
use std::sync::atomic::{AtomicUsize, Ordering};
use chrono::Utc;

static TASK_COUNTER: AtomicUsize = AtomicUsize::new(0);

pub struct NewTaskHandler;
impl NewTaskHandler {
    pub fn new() -> Self { Self }
}

pub struct NewTaskTool;
impl NewTaskTool {
    pub fn new() -> Self { Self }
}
#[async_trait]
impl Tool for NewTaskTool {
    fn name(&self) -> &str { "newTask" }
    fn description(&self) -> &str { "Create a new task with tracking" }
    fn parameters(&self) -> Vec<crate::mcp_tools::core::ToolParameter> { vec![] }
    fn input_schema(&self) -> Value { 
        json!({
            "type": "object",
            "properties": {
                "title": {
                    "type": "string",
                    "description": "Task title"
                },
                "description": {
                    "type": "string",
                    "description": "Task description"
                },
                "priority": {
                    "type": "string",
                    "enum": ["low", "medium", "high", "critical"],
                    "description": "Task priority"
                },
                "assignee": {
                    "type": "string",
                    "description": "Task assignee"
                },
                "due_date": {
                    "type": "string",
                    "description": "Due date (ISO format)"
                },
                "tags": {
                    "type": "array",
                    "items": {"type": "string"},
                    "description": "Task tags"
                }
            },
            "required": ["title"]
        })
    }
    
    async fn validate(&self, args: &Value) -> Result<()> {
        if !args.is_object() || args.get("title").is_none() {
            anyhow::bail!("Missing required parameter: title");
        }
        Ok(())
    }
    
    async fn execute(&self, args: Value, _context: ToolContext) -> Result<ToolResult> {
        let (title, description, priority, assignee, due_date, tags) = if let Some(xml_str) = args.as_str() {
            let tool_use = crate::mcp_tools::xml::parse_tool_use(xml_str)?;
            let title = tool_use.params.get("title")
                .ok_or_else(|| anyhow::anyhow!("Missing title in XML"))?
                .clone();
            let description = tool_use.params.get("description")
                .map(|s| s.clone())
                .unwrap_or_default();
            let priority = tool_use.params.get("priority")
                .cloned()
                .unwrap_or_else(|| "medium".to_string());
            let assignee = tool_use.params.get("assignee").map(|s| s.clone());
            let due_date = tool_use.params.get("due_date").map(|s| s.clone());
            let tags: Vec<String> = tool_use.params.get("tags")
                .and_then(|s| serde_json::from_str(s).ok())
                .unwrap_or_default();
            (title, description, priority, assignee, due_date, tags)
        } else {
            let title = args.get("title")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("Invalid title parameter"))?
                .to_string();
            let description = args.get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let priority = args.get("priority")
                .and_then(|v| v.as_str())
                .unwrap_or("medium")
                .to_string();
            let assignee = args.get("assignee")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let due_date = args.get("due_date")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let tags: Vec<String> = args.get("tags")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
                .unwrap_or_default();
            (title, description, priority, assignee, due_date, tags)
        };
        
        // Generate task ID
        let task_id = format!("TASK-{}", TASK_COUNTER.fetch_add(1, Ordering::Relaxed) + 1);
        let created_at = Utc::now().to_rfc3339();
        // Build task data
        let mut task_xml = format!(
            "<task><id>{}</id><title>{}</title><description>{}</description><priority>{}</priority><status>open</status><created_at>{}</created_at>",
            task_id,
            html_escape::encode_text(&title),
            html_escape::encode_text(&description),
            priority,
            created_at
        );
        if let Some(assignee) = assignee {
            task_xml.push_str(&format!("<assignee>{}</assignee>", html_escape::encode_text(&assignee)));
        }
        if let Some(due_date) = due_date {
            task_xml.push_str(&format!("<due_date>{}</due_date>", due_date));
        }
        if !tags.is_empty() {
            task_xml.push_str("<tags>");
            for tag in tags {
                task_xml.push_str(&format!("<tag>{}</tag>", html_escape::encode_text(&tag)));
            }
            task_xml.push_str("</tags>");
        }
        task_xml.push_str("</task>");
        let xml_response = format!(
            "<tool_result><success>true</success>{}</tool_result>",
            task_xml
        );
        Ok(ToolResult::from_xml(xml_response))
    }
    
    fn required_permissions(&self) -> Vec<Permission> {
        vec![Permission::FileWrite("*".to_string()), Permission::FileRead("*".to_string())]
    }
}
