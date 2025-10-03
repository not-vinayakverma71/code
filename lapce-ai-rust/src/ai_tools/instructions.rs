use crate::types_tool::ToolParameter;
use crate::mcp_tools::{core::{Tool, ToolContext, ToolResult, JsonSchema, ResourceLimits}, permissions::Permission};
use async_trait::async_trait;
use serde_json::{json, Value};
use anyhow::Result;
use std::path::PathBuf;
use tokio::fs;

pub struct FetchInstructionsTool;
impl FetchInstructionsTool {
    pub fn new() -> Self { Self }
}
#[async_trait]
impl Tool for FetchInstructionsTool {
    fn name(&self) -> &str { "fetchInstructions" }
    fn description(&self) -> &str { "Fetch instructions from configuration or documentation files" }
    fn parameters(&self) -> Vec<crate::mcp_tools::core::ToolParameter> { vec![] }
    fn input_schema(&self) -> Value { 
        json!({
            "type": "object",
            "properties": {
                "source": {
                    "type": "string",
                    "description": "Source of instructions (file path or type)"
                },
                "format": {
                    "enum": ["markdown", "json", "yaml", "text"],
                    "description": "Format of instructions"
                },
                "section": {
                    "type": "string",
                    "description": "Specific section to fetch"
                }
            },
            "required": ["source"]
        })
    }
    
    async fn validate(&self, args: &Value) -> Result<()> {
        if !args.is_object() || args.get("source").is_none() {
            anyhow::bail!("Missing required parameter: source");
        }
        Ok(())
    }
    
    async fn execute(&self, args: Value, context: ToolContext) -> Result<ToolResult> {
        let (source, format, section) = if let Some(xml_str) = args.as_str() {
            let tool_use = crate::mcp_tools::xml::parse_tool_use(xml_str)?;
            let source = tool_use.params.get("source")
                .ok_or_else(|| anyhow::anyhow!("Missing source in XML"))?
                .clone();
            let format = tool_use.params.get("format")
                .map(|s| s.clone())
                .unwrap_or_else(|| "text".to_string());
            let section = tool_use.params.get("section").map(|s| s.clone());
            (source, format, section)
        } else {
            let source = args.get("source")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("Invalid source"))?
                .to_string();
            let format = args.get("format")
                .and_then(|v| v.as_str())
                .unwrap_or("text")
                .to_string();
            let section = args.get("section")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            (source, format, section)
        };
        
        // Determine the instructions source
        let instructions = if source.starts_with("http") {
            // Fetch from URL
            match reqwest::get(&source).await {
                Ok(resp) => resp.text().await.unwrap_or_default(),
                Err(e) => {
                    return Ok(ToolResult::from_xml(format!(
                        "<tool_result><success>false</success><error>Failed to fetch from URL: {}</error></tool_result>",
                        e
                    )));
                }
            }
        } else {
            // Try as file path
            let path = if source.starts_with('/') {
                PathBuf::from(&source)
            } else {
                context.workspace.join(&source)
            };
            
            // Look for common instruction files if path doesn't exist
            let instruction_paths = if !path.exists() {
                vec![
                    context.workspace.join("README.md"),
                    context.workspace.join("INSTRUCTIONS.md"),
                    context.workspace.join("docs/INSTRUCTIONS.md"),
                    context.workspace.join(".instructions"),
                    context.workspace.join("config/instructions.json"),
                ]
            } else {
                vec![path]
            };
            
            let mut content = String::new();
            for p in instruction_paths {
                if p.exists() {
                    content = fs::read_to_string(&p).await.unwrap_or_default();
                    if !content.is_empty() {
                        break;
                    }
                }
            }
            
            if content.is_empty() {
                // Return default instructions
                content = "No specific instructions found. Default instructions:\n\
                          1. Follow best practices\n\
                          2. Write clean, maintainable code\n\
                          3. Document your changes\n\
                          4. Test thoroughly\n\
                          5. Handle errors gracefully".to_string();
            }
            content
        };
        
        // Extract section if specified
        let final_instructions = if let Some(sect) = section {
            extract_section(&instructions, &sect, &format)
        } else {
            instructions
        };
        
        let xml_response = format!(
            "<tool_result><success>true</success><source>{}</source><format>{}</format><instructions>{}</instructions></tool_result>",
            html_escape::encode_text(&source),
            format,
            html_escape::encode_text(&final_instructions)
        );
        Ok(ToolResult::from_xml(xml_response))
    }
    
    fn required_permissions(&self) -> Vec<Permission> { 
        vec![Permission::FileRead("*".to_string())]
    }
}

fn extract_section(content: &str, section: &str, format: &str) -> String {
    match format {
        "markdown" => {
            // Extract markdown section
            let header = format!("# {}", section);
            if let Some(start) = content.find(&header) {
                let section_content = &content[start..];
                if let Some(next_header) = section_content[header.len()..].find("\n#") {
                    section_content[..header.len() + next_header].to_string()
                } else {
                    section_content.to_string()
                }
            } else {
                content.to_string()
            }
        }
        _ => content.to_string()
    }
}
