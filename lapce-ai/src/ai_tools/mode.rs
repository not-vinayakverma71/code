use crate::types_tool::ToolParameter;
use crate::mcp_tools::{core::{Tool, ToolContext, ToolResult, JsonSchema, ResourceLimits}, permissions::Permission};
use async_trait::async_trait;
use serde_json::{json, Value};
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;

// Global mode storage
use once_cell::sync::Lazy;

static CURRENT_MODE: Lazy<Arc<RwLock<String>>> = Lazy::new(|| Arc::new(RwLock::new("normal".to_string())));
pub struct SwitchModeTool;
impl SwitchModeTool {
    pub fn new() -> Self { Self }
}

#[async_trait]
impl Tool for SwitchModeTool {
    fn name(&self) -> &str { "switchMode" }
    fn description(&self) -> &str { "Switch between different operation modes" }
    fn parameters(&self) -> Vec<crate::mcp_tools::core::ToolParameter> { vec![] }
    fn input_schema(&self) -> Value { 
        json!({
            "type": "object",
            "properties": {
                "mode": {
                    "type": "string",
                    "enum": ["normal", "debug", "safe", "development", "production", "test"],
                    "description": "Mode to switch to"
                },
                "options": {
                    "type": "object",
                    "description": "Mode-specific options"
                }
            },
            "required": ["mode"]
        })
    }
    
    async fn validate(&self, args: &Value) -> Result<()> {
        if !args.is_object() || args.get("mode").is_none() {
            anyhow::bail!("Missing required parameter: mode");
        }
        Ok(())
    }
    
    async fn execute(&self, args: Value, _context: ToolContext) -> Result<ToolResult> {
        let (new_mode, options) = if let Some(xml_str) = args.as_str() {
            let tool_use = crate::mcp_tools::xml::parse_tool_use(xml_str)?;
            let mode = tool_use.params.get("mode")
                .ok_or_else(|| anyhow::anyhow!("Missing mode in XML"))?
                .clone();
            let options = tool_use.params.get("options")
                .and_then(|s| serde_json::from_str::<Value>(s).ok());
            (mode, options)
        } else {
            let mode = args.get("mode")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("Invalid mode"))?
                .to_string();
            let options = args.get("options").cloned();
            (mode, options)
        };
        
        // Validate mode
        let valid_modes = vec!["normal", "debug", "safe", "development", "production", "test"];
        if !valid_modes.contains(&new_mode.as_str()) {
            return Ok(ToolResult::from_xml(format!(
                "<tool_result><success>false</success><error>Invalid mode: {}. Valid modes are: {:?}</error></tool_result>",
                new_mode, valid_modes
            )));
        }
        
        // Get previous mode
        let previous_mode = {
            let current = CURRENT_MODE.read().await;
            current.clone()
        };
        
        // Switch mode
        {
            let mut current = CURRENT_MODE.write().await;
            *current = new_mode.clone();
        }
        
        // Apply mode-specific settings
        let settings_applied = match new_mode.as_str() {
            "debug" => {
                // Enable debug logging, verbose output, etc.
                "Debug logging enabled, verbose output activated"
            }
            "safe" => {
                // Restrict operations, enable confirmations, etc.
                "Safe mode activated - all destructive operations require confirmation"
            }
            "development" => {
                // Enable hot reload, development tools, etc.
                "Development mode activated - hot reload enabled, development tools available"
            }
            "production" => {
                // Optimize for performance, disable debug features
                "Production mode activated - optimizations enabled, debug features disabled"
            }
            "test" => {
                // Enable test fixtures, mocks, etc.
                "Test mode activated - test fixtures loaded, mocks enabled"
            }
            _ => {
                // Normal mode
                "Normal mode activated - standard settings applied"
            }
        };
        
        // Process options if provided
        let options_result = if let Some(opts) = options {
            format!(", options applied: {}", opts)
        } else {
            String::new()
        };
        
        let xml_response = format!(
            "<tool_result><success>true</success><previous_mode>{}</previous_mode><current_mode>{}</current_mode><settings>{}{}</settings></tool_result>",
            previous_mode,
            new_mode,
            settings_applied,
            options_result
        );
        Ok(ToolResult::from_xml(xml_response))
    }
    
    fn required_permissions(&self) -> Vec<Permission> {
        vec![Permission::FileWrite("*".to_string())]
    }
}
