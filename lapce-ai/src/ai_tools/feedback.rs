use crate::mcp_tools::{core::{Tool, ToolContext, ToolResult}, permissions::Permission};
use async_trait::async_trait;
use serde_json::{json, Value};
use anyhow::Result;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::path::PathBuf;
use tokio::fs;
use chrono::Utc;

static BUG_COUNTER: AtomicUsize = AtomicUsize::new(0);
pub struct ReportBugTool;
impl ReportBugTool {
    pub fn new() -> Self { Self }
}
#[async_trait]
impl Tool for ReportBugTool {
    fn name(&self) -> &str { "reportBug" }
    fn description(&self) -> &str { "Report a bug or issue with detailed information" }
    fn parameters(&self) -> Vec<crate::mcp_tools::core::ToolParameter> { vec![] }
    fn input_schema(&self) -> Value { 
        json!({
            "type": "object",
            "properties": {
                "title": {
                    "type": "string",
                    "description": "Bug title/summary"
                },
                "description": {
                    "type": "string",
                    "description": "Detailed bug description"
                },
                "severity": {
                    "type": "string",
                    "enum": ["low", "medium", "high", "critical"],
                    "description": "Bug severity"
                },
                "category": {
                    "type": "string",
                    "enum": ["functionality", "performance", "security", "ui", "documentation", "other"],
                    "description": "Bug category"
                },
                "steps_to_reproduce": {
                    "type": "array",
                    "items": {"type": "string"},
                    "description": "Steps to reproduce the bug"
                },
                "expected_behavior": {
                    "type": "string",
                    "description": "Expected behavior"
                },
                "actual_behavior": {
                    "type": "string",
                    "description": "Actual behavior observed"
                },
                "environment": {
                    "type": "object",
                    "description": "Environment information"
                },
                "save_to_file": {
                    "type": "string",
                    "description": "Optional file path to save bug report"
                }
            },
            "required": ["title", "description"]
        })
    }
    
    async fn validate(&self, args: &Value) -> Result<()> {
        if !args.is_object() || args.get("title").is_none() || args.get("description").is_none() {
            anyhow::bail!("Missing required parameters: title and description");
        }
        Ok(())
    }
    
    async fn execute(&self, args: Value, context: ToolContext) -> Result<ToolResult> {
        let (title, description, severity, category, steps, expected, actual, env_info, save_path) = 
            if let Some(xml_str) = args.as_str() {
                let tool_use = crate::mcp_tools::xml::parse_tool_use(xml_str)?;
                let title = tool_use.params.get("title")
                    .ok_or_else(|| anyhow::anyhow!("Missing title in XML"))?
                    .clone();
                let description = tool_use.params.get("description")
                    .ok_or_else(|| anyhow::anyhow!("Missing description in XML"))?
                    .clone();
                let severity = tool_use.params.get("severity")
                    .map(|s| s.clone())
                    .unwrap_or_else(|| "medium".to_string());
                let category = tool_use.params.get("category")
                    .cloned()
                    .unwrap_or_else(|| "functionality".to_string());
                let steps: Vec<String> = tool_use.params.get("steps_to_reproduce")
                    .and_then(|s| serde_json::from_str(s).ok())
                    .unwrap_or_default();
                let expected = tool_use.params.get("expected_behavior")
                    .map(|s| s.clone())
                    .unwrap_or_default();
                let actual = tool_use.params.get("actual_behavior")
                    .map(|s| s.clone())
                    .unwrap_or_default();
                let env_info: Option<Value> = tool_use.params.get("environment")
                    .and_then(|s| serde_json::from_str(s).ok());
                let save_path = tool_use.params.get("save_to_file").map(|s| s.clone());
                (title, description, severity, category, steps, expected, actual, env_info, save_path)
            } else {
                let title = args.get("title")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Invalid title"))?
                    .to_string();
                let description = args.get("description")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Invalid description"))?
                    .to_string();
                let severity = args.get("severity")
                    .and_then(|v| v.as_str())
                    .unwrap_or("medium")
                    .to_string();
                let category = args.get("category")
                    .and_then(|v| v.as_str())
                    .unwrap_or("functionality")
                    .to_string();
                let steps: Vec<String> = args.get("steps_to_reproduce")
                    .and_then(|v| v.as_array())
                    .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
                    .unwrap_or_else(Vec::new);
                let expected = args.get("expected_behavior")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let actual = args.get("actual_behavior")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let env_info = args.get("environment").cloned();
                let save_path = args.get("save_to_file")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                (title, description, severity, category, steps, expected, actual, env_info, save_path)
            };
        
        // Generate bug ID
        let bug_id = format!("BUG-{}", BUG_COUNTER.fetch_add(1, Ordering::Relaxed) + 1);
        let timestamp = Utc::now().to_rfc3339();
        // Build bug report
        let mut report = format!(
            r#"# Bug Report: {}
**ID**: {}
**Date**: {}
**Severity**: {}
**Category**: {}
**Status**: Open
## Description
{}
## Steps to Reproduce"#,
            title, bug_id, timestamp, severity, category, description
        );
        if !steps.is_empty() {
            for (i, step) in steps.iter().enumerate() {
                report.push_str(&format!("\n{}. {}", i + 1, step));
            }
        } else {
            report.push_str("\nNo steps provided");
        }
        
        if !expected.is_empty() {
            report.push_str(&format!("\n\n## Expected Behavior\n{}", expected));
        }
        
        if !actual.is_empty() {
            report.push_str(&format!("\n\n## Actual Behavior\n{}", actual));
        }
        
        if let Some(env) = env_info {
            report.push_str(&format!("\n\n## Environment\n```json\n{}\n```", 
                serde_json::to_string_pretty(&env).unwrap_or_default()));
        }
        
        // Save to file if requested
        if let Some(path_str) = save_path {
            let path = if path_str.starts_with('/') {
                PathBuf::from(&path_str)
            } else {
                context.workspace.join(&path_str)
            };
            
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).await?;
            }
            fs::write(&path, &report).await?;
        }
        let xml_response = format!(
            "<tool_result><success>true</success><bug><id>{}</id><title>{}</title><severity>{}</severity><category>{}</category><timestamp>{}</timestamp></bug></tool_result>",
            bug_id,
            html_escape::encode_text(&title),
            severity,
            category,
            timestamp
        );
        Ok(ToolResult::from_xml(xml_response))
    }
    
    fn required_permissions(&self) -> Vec<Permission> {
        vec![Permission::FileWrite("*".to_string())]
    }
}
