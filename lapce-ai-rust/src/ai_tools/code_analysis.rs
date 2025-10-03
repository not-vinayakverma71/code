use crate::types_tool::ToolParameter;

pub struct CodeAnalyzer;
impl CodeAnalyzer {
    pub fn new() -> Self { Self }
}
use crate::mcp_tools::{core::{Tool, ToolContext, ToolResult, JsonSchema, ResourceLimits}, permissions::Permission};
use async_trait::async_trait;
use serde_json::{json, Value};
use anyhow::Result;
use std::path::{Path, PathBuf};
use tokio::fs;

pub struct ListCodeDefinitionsTool;
impl ListCodeDefinitionsTool {
    pub fn new() -> Self { Self }
}
#[async_trait]
impl Tool for ListCodeDefinitionsTool {
    fn name(&self) -> &str { "listCodeDefinitions" }
    fn description(&self) -> &str { "List code definitions (functions, classes, structs) in files" }
    fn parameters(&self) -> Vec<crate::mcp_tools::core::ToolParameter> { vec![] }
    fn input_schema(&self) -> Value { 
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "File or directory path to analyze"
                },
                "language": {
                    "description": "Programming language (rust, python, javascript)"
                }
            },
            "required": ["path"]
        })
    }
    
    async fn validate(&self, args: &Value) -> Result<()> {
        Ok(())
    }
    
    async fn execute(&self, args: Value, context: ToolContext) -> Result<ToolResult> {
        let (path_str, language) = if let Some(xml_str) = args.as_str() {
            let tool_use = crate::mcp_tools::xml::parse_tool_use(xml_str)?;
            let path = tool_use.params.get("path")
                .ok_or_else(|| anyhow::anyhow!("Missing path in XML"))?
                .clone();
            let lang = tool_use.params.get("language")
                .map(|s| s.clone())
                .unwrap_or_else(|| "rust".to_string());
            (path, lang)
        } else {
            let path = args.get("path")
                .and_then(|v| v.as_str())
                .unwrap_or(".")
                .to_string();
            let lang = args.get("language")
                .and_then(|v| v.as_str())
                .unwrap_or("rust")
                .to_string();
            (path, lang)
        };
        
        let path = if Path::new(&path_str).is_absolute() {
            PathBuf::from(&path_str)
        } else {
            context.workspace.join(&path_str)
        };
        
        if !path.exists() {
            let xml_response = format!(
                "<tool_result><success>false</success><error>Path not found: {}</error></tool_result>",
                path.display()
            );
            return Ok(ToolResult::from_xml(xml_response));
        }
        let mut definitions = Vec::new();
        if path.is_file() {
            if let Ok(content) = fs::read_to_string(&path).await.map(|s| s.to_string()) {
                definitions = extract_definitions(&content, &language);
            }
        } else if path.is_dir() {
            // Scan directory for code files
            if let Ok(mut entries) = fs::read_dir(&path).await {
                while let Some(entry) = entries.next_entry().await? {
                    if entry.path().is_file() {
                        if let Some(ext) = entry.path().extension() {
                            let is_code_file = match ext.to_str() {
                                Some("rs") | Some("py") | Some("js") | Some("ts") => true,
                                _ => false
                            };
                            if is_code_file {
                                if let Ok(content) = fs::read_to_string(entry.path()).await.map(|s| s.to_string()) {
                                    let file_defs = extract_definitions(&content, &language);
                                    for def in file_defs {
                                        definitions.push(format!("{}:{}", entry.path().display(), def));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        
        let mut xml_defs = String::from("<definitions>");
        for def in &definitions {
            xml_defs.push_str(&format!("<definition>{}</definition>", html_escape::encode_text(def)));
        }
        xml_defs.push_str("</definitions>");
        let xml_response = format!(
            "<tool_result><success>true</success><path>{}</path>{}<count>{}</count></tool_result>",
            path.display(),
            xml_defs,
            definitions.len()
        );
        Ok(ToolResult::from_xml(xml_response))
    }
    
    fn required_permissions(&self) -> Vec<Permission> { 
        vec![Permission::FileRead("*".to_string()), Permission::FileWrite("*".to_string())] 
    }
}

fn extract_definitions(content: &str, language: &str) -> Vec<String> {
    let mut definitions = Vec::new();
    match language {
        "rust" => {
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("pub fn ") || trimmed.starts_with("fn ") {
                    if let Some(name) = extract_rust_function_name(trimmed) {
                        definitions.push(format!("fn {}", name));
                    }
                } else if trimmed.starts_with("pub struct ") || trimmed.starts_with("struct ") {
                    if let Some(name) = extract_rust_type_name(trimmed, "struct") {
                        definitions.push(format!("struct {}", name));
                    }
                } else if trimmed.starts_with("pub enum ") || trimmed.starts_with("enum ") {
                    if let Some(name) = extract_rust_type_name(trimmed, "enum") {
                        definitions.push(format!("enum {}", name));
                    }
                } else if trimmed.starts_with("pub trait ") || trimmed.starts_with("trait ") {
                    if let Some(name) = extract_rust_type_name(trimmed, "trait") {
                        definitions.push(format!("trait {}", name));
                    }
                }
            }
        }
        "python" => {
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("def ") {
                    if let Some(name) = extract_python_function_name(trimmed) {
                        definitions.push(format!("def {}", name));
                    }
                } else if trimmed.starts_with("class ") {
                    if let Some(name) = extract_python_class_name(trimmed) {
                        definitions.push(format!("class {}", name));
                    }
                }
            }
        }
        _ => {
            // Basic extraction for other languages
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.contains("function ") || trimmed.contains("class ") {
                    definitions.push(trimmed.to_string());
                }
            }
        }
    }
    definitions
}

fn extract_rust_function_name(line: &str) -> Option<String> {
    let without_pub = if line.starts_with("pub ") {
        &line[4..]
    } else {
        line
    };
    if let Some(start) = without_pub.find("fn ") {
        let after_fn = &without_pub[start + 3..];
        if let Some(end) = after_fn.find(|c: char| c == '(' || c == '<') {
            return Some(after_fn[..end].trim().to_string());
        }
    }
    None
}

fn extract_rust_type_name(line: &str, type_keyword: &str) -> Option<String> {
    let pattern = format!("{} ", type_keyword);
    if let Some(start) = line.find(&pattern) {
        let after_keyword = &line[start + pattern.len()..];
        if let Some(end) = after_keyword.find(|c: char| c == ' ' || c == '<' || c == '{' || c == ';') {
            return Some(after_keyword[..end].trim().to_string());
        }
    }
    None
}

fn extract_python_function_name(line: &str) -> Option<String> {
    if let Some(start) = line.find("def ") {
        let after_def = &line[start + 4..];
        if let Some(end) = after_def.find('(') {
            return Some(after_def[..end].trim().to_string());
        }
    }
    None
}

fn extract_python_class_name(line: &str) -> Option<String> {
    if let Some(start) = line.find("class ") {
        let after_class = &line[start + 6..];
        if let Some(end) = after_class.find(|c: char| c == '(' || c == ':') {
            return Some(after_class[..end].trim().to_string());
        }
    }
    None
}
