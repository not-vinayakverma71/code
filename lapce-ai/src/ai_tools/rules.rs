use crate::mcp_tools::{
    core::{Tool, ToolContext, ToolResult, ResourceLimits},
    permissions::Permission,
};
use async_trait::async_trait;
use serde_json::{json, Value};
use anyhow::Result;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::path::PathBuf;
use tokio::fs;

static RULE_COUNTER: AtomicUsize = AtomicUsize::new(0);
pub struct NewRuleTool;
impl NewRuleTool {
    pub fn new() -> Self { Self }
}
#[async_trait]
impl Tool for NewRuleTool {
    fn name(&self) -> &str { "newRule" }
    fn description(&self) -> &str { "Create a new rule for automation or validation" }
    fn parameters(&self) -> Vec<crate::mcp_tools::core::ToolParameter> { vec![] }
    fn input_schema(&self) -> Value { 
        json!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "description": "Rule name"
                },
                "type": {
                    "type": "string",
                    "enum": ["validation", "automation", "security", "style", "performance"],
                    "description": "Rule type"
                },
                "condition": {
                    "type": "string",
                    "description": "Rule condition expression"
                },
                "action": {
                    "type": "string",
                    "description": "Action to take when rule triggers"
                },
                "severity": {
                    "enum": ["info", "warning", "error", "critical"],
                    "description": "Rule severity"
                },
                "enabled": {
                    "type": "boolean",
                    "description": "Whether rule is enabled"
                },
                "save_to_file": {
                    "type": "string",
                    "description": "Optional file path to save rule"
                }
            },
            "required": ["name", "type", "condition", "action"]
        })
    }
    
    async fn validate(&self, args: &Value) -> Result<()> {
        if !args.is_object() || 
           args.get("name").is_none() || 
           args.get("type").is_none() ||
           args.get("condition").is_none() ||
           args.get("action").is_none() {
            anyhow::bail!("Missing required parameters");
        }
        Ok(())
    }
    
    async fn execute(&self, args: Value, context: ToolContext) -> Result<ToolResult> {
        let (name, rule_type, condition, action, severity, enabled, save_path) = if let Some(xml_str) = args.as_str() {
            let tool_use = crate::mcp_tools::xml::parse_tool_use(xml_str)?;
            let name = tool_use.params.get("name")
                .ok_or_else(|| anyhow::anyhow!("Missing name in XML"))?
                .clone();
            let rule_type = tool_use.params.get("type")
                .ok_or_else(|| anyhow::anyhow!("Missing type in XML"))?
                .clone();
            let condition = tool_use.params.get("condition")
                .ok_or_else(|| anyhow::anyhow!("Missing condition in XML"))?
                .clone();
            let action = tool_use.params.get("action")
                .ok_or_else(|| anyhow::anyhow!("Missing action in XML"))?
                .clone();
            let severity = tool_use.params.get("severity")
                .map(|s| s.clone())
                .unwrap_or_else(|| "warning".to_string());
            let enabled = tool_use.params.get("enabled")
                .and_then(|s| s.parse::<bool>().ok())
                .unwrap_or(true);
            let save_path = tool_use.params.get("save_to_file").map(|s| s.clone());
            (name, rule_type, condition, action, severity, enabled, save_path)
        } else {
            let name = args.get("name")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("Invalid name"))?
                .to_string();
            let rule_type = args.get("type")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("Invalid type"))?
                .to_string();
            let condition = args.get("condition")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("Invalid condition"))?
                .to_string();
            let action = args.get("action")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("Invalid action"))?
                .to_string();
            let severity = args.get("severity")
                .and_then(|v| v.as_str())
                .unwrap_or("warning")
                .to_string();
            let enabled = args.get("enabled")
                .and_then(|v| v.as_bool())
                .unwrap_or(true);
            let save_path = args.get("save_to_file")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            (name, rule_type, condition, action, severity, enabled, save_path)
        };
        
        // Generate rule ID
        let rule_id = format!("RULE-{}", RULE_COUNTER.fetch_add(1, Ordering::Relaxed) + 1);
        // Build rule definition
        let rule_def = format!(
            r#"# Rule: {}
ID: {}
Type: {}
Severity: {}
Enabled: {}
## Condition
{}
## Action
{}
## Metadata
Created: {}
Status: active"#,
            name,
            rule_id,
            rule_type,
            severity,
            enabled,
            condition,
            action,
            chrono::Utc::now().to_rfc3339()
        );
        // Save to file if requested
        if let Some(path_str) = save_path {
            let path = if path_str.starts_with('/') {
                PathBuf::from(&path_str)
            } else {
                context.workspace.join(&path_str)
            };
            
            // Ensure directory exists
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).await?;
            }
            fs::write(&path, &rule_def).await?;
        }
        
        let xml_response = format!(
            "<tool_result><success>true</success><rule><id>{}</id><name>{}</name><type>{}</type><severity>{}</severity><enabled>{}</enabled><condition>{}</condition><action>{}</action></rule></tool_result>",
            rule_id,
            html_escape::encode_text(&name),
            html_escape::encode_text(&rule_type),
            severity,
            enabled,
            html_escape::encode_text(&condition),
            html_escape::encode_text(&action)
        );
        Ok(ToolResult::from_xml(xml_response))
    }
    
    fn required_permissions(&self) -> Vec<Permission> { 
        vec![Permission::FileWrite("*".to_string())]
    }
    
    fn resource_limits(&self) -> ResourceLimits {
        ResourceLimits::default()
    }
}
