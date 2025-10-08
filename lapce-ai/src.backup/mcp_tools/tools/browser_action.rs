use crate::mcp_tools::{core::{Tool, ToolContext, ToolResult}, permissions::Permission};
use async_trait::async_trait;
use serde_json::{json, Value};
use anyhow::Result;
use std::process::Command;

pub struct BrowserActionTool;
impl BrowserActionTool {
    pub fn new() -> Self { Self }
}
#[async_trait]
impl Tool for BrowserActionTool {
    fn name(&self) -> &str { "browserAction" }
    fn description(&self) -> &str { "Control browser actions like opening URLs, taking screenshots" }
    fn parameters(&self) -> Vec<crate::mcp_tools::core::ToolParameter> { vec![] }
    fn input_schema(&self) -> Value { 
        json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["open", "screenshot", "close", "refresh"],
                    "description": "Browser action to perform"
                },
                "url": {
                    "type": "string",
                    "description": "URL for open action"
                },
                "path": {
                    "type": "string", 
                    "description": "File path for screenshot"
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
    
    async fn execute(&self, args: Value, _context: ToolContext) -> Result<ToolResult> {
        let (action, url, path) = if let Some(xml_str) = args.as_str() {
            let tool_use = crate::mcp_tools::xml::parse_tool_use(xml_str)?;
            let action = tool_use.params.get("action")
                .ok_or_else(|| anyhow::anyhow!("Missing action in XML"))?
                .clone();
            let url = tool_use.params.get("url").map(|s| s.clone());
            let path = tool_use.params.get("path").map(|s| s.clone());
            (action, url, path)
        } else {
            let action = args.get("action")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("Invalid action"))?
                .to_string();
            let url = args.get("url")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let path = args.get("path")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            (action, url, path)
        };
        
        let result = match action.as_str() {
            "open" => {
                let url = url.ok_or_else(|| anyhow::anyhow!("Missing URL for open action"))?;
                // Use xdg-open on Linux, open on macOS, start on Windows
                let output = if cfg!(target_os = "linux") {
                    Command::new("xdg-open").arg(&url).output()
                } else if cfg!(target_os = "macos") {
                    Command::new("open").arg(&url).output()
                } else {
                    Command::new("cmd").args(&["/C", "start", &url]).output()
                };
                
                match output {
                    Ok(_) => format!("Opened URL: {}", url),
                    Err(e) => return Ok(ToolResult::from_xml(format!(
                        "<tool_result><success>false</success><error>Failed to open browser: {}</error></tool_result>",
                        e
                    )))
                }
            }
            "screenshot" => {
                let path = path.unwrap_or_else(|| "screenshot.png".to_string());
                // Simple screenshot using available tools
                let output = if cfg!(target_os = "linux") {
                    Command::new("gnome-screenshot").args(&["-f", &path]).output()
                } else if cfg!(target_os = "macos") {
                    Command::new("screencapture").arg(&path).output()
                } else {
                    return Ok(ToolResult::from_xml(
                        "<tool_result><success>false</success><error>Screenshot not supported on this platform</error></tool_result>".to_string()
                    ));
                };
                
                match output {
                    Ok(_) => format!("Screenshot saved to: {}", path),
                    Err(e) => return Ok(ToolResult::from_xml(format!(
                        "<tool_result><success>false</success><error>Failed to take screenshot: {}</error></tool_result>",
                        e
                    )))
                }
            }
            "refresh" => {
                "Browser refresh simulated".to_string()
            }
            "close" => {
                "Browser close simulated".to_string()
            }
            _ => {
                return Ok(ToolResult::from_xml(format!(
                    "<tool_result><success>false</success><error>Unknown action: {}</error></tool_result>",
                    action
                )));
            }
        };
        
        let xml_response = format!(
            "<tool_result><success>true</success><action>{}</action><result>{}</result></tool_result>",
            action,
            result
        );
        Ok(ToolResult::from_xml(xml_response))
    }
    
    fn required_permissions(&self) -> Vec<Permission> { 
        vec![Permission::FileWrite("*".to_string())]
    }
}
