use crate::mcp_tools::{
    core::{Tool, ToolContext, ToolResult, ToolParameter, ResourceLimits},
    permissions::Permission,
};
use async_trait::async_trait;
use anyhow::Result;
use serde_json::{json, Value};
use std::path::PathBuf;
use walkdir::WalkDir;
use regex::Regex;
use tokio::fs;

pub struct SearchAndReplaceTool;
impl SearchAndReplaceTool {
    pub fn new() -> Self { Self }
}
#[async_trait]
impl Tool for SearchAndReplaceTool {
    fn name(&self) -> &str { "searchAndReplace" }
    fn description(&self) -> &str { "Search and replace text across multiple files" }
    fn parameters(&self) -> Vec<ToolParameter> { vec![] }
    fn input_schema(&self) -> Value { 
        json!({
            "type": "object",
            "properties": {
                "search": {
                    "type": "string",
                    "description": "Text or regex pattern to search for"
                },
                "replace": {
                    "type": "string",
                    "description": "Text to replace with"
                },
                "path": {
                    "type": "string",
                    "description": "File or directory path"
                },
                "file_types": {
                    "type": "array",
                    "items": {"type": "string"},
                    "description": "File extensions to include"
                },
                "use_regex": {
                    "type": "boolean",
                    "description": "Use regex for search"
                },
                "dry_run": {
                    "type": "boolean",
                    "description": "Preview changes without applying"
                }
            },
            "required": ["search", "replace"]
        })
    }
    
    async fn validate(&self, args: &Value) -> Result<()> {
        if !args.is_object() || args.get("search").is_none() || args.get("replace").is_none() {
            anyhow::bail!("Missing required parameters: search and replace");
        }
        Ok(())
    }
    
    async fn execute(&self, args: Value, context: ToolContext) -> Result<ToolResult> {
        let (search, replace, path, file_types, use_regex, dry_run) = if let Some(xml_str) = args.as_str() {
            let tool_use = crate::mcp_tools::xml::parse_tool_use(xml_str)?;
            let search = tool_use.params.get("search")
                .ok_or_else(|| anyhow::anyhow!("Missing search in XML"))?
                .clone();
            let replace = tool_use.params.get("replace")
                .ok_or_else(|| anyhow::anyhow!("Missing replace in XML"))?
                .clone();
            let path = tool_use.params.get("path")
                .map(|s| PathBuf::from(s))
                .unwrap_or_else(|| context.workspace.clone());
            let file_types: Vec<String> = tool_use.params.get("file_types")
                .and_then(|s| serde_json::from_str(s).ok())
                .unwrap_or_else(Vec::new);
            let use_regex = tool_use.params.get("use_regex")
                .and_then(|s| s.parse::<bool>().ok())
                .unwrap_or(false);
            let dry_run = tool_use.params.get("dry_run")
                .and_then(|s| s.parse::<bool>().ok())
                .unwrap_or(false);
            (search, replace, path, file_types, use_regex, dry_run)
        } else {
            let search = args.get("search")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("Invalid search parameter"))?
                .to_string();
            let replace = args.get("replace")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("Invalid replace parameter"))?
                .to_string();
            let path = args.get("path")
                .and_then(|v| v.as_str())
                .map(|s| PathBuf::from(s))
                .unwrap_or_else(|| context.workspace.clone());
            let file_types: Vec<String> = args.get("file_types")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
                .unwrap_or_default();
            let use_regex = args.get("use_regex")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let dry_run = args.get("dry_run")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            (search, replace, path, file_types, use_regex, dry_run)
        };
        
        if !path.exists() {
            let xml_response = format!(
                "<tool_result><success>false</success><error>Path not found: {}</error></tool_result>",
                path.display()
            );
            return Ok(ToolResult::from_xml(xml_response));
        }
        
        // Compile regex if needed
        let regex = if use_regex {
            match Regex::new(&search) {
                Ok(r) => Some(r),
                Err(e) => {
                    let xml_response = format!(
                        "<tool_result><success>false</success><error>Invalid regex: {}</error></tool_result>",
                        e
                    );
                    return Ok(ToolResult::from_xml(xml_response));
                }
            }
        } else {
            None
        };
        
        let mut xml_results = String::from("<files>");
        let mut total_replacements = 0;
        let mut files_modified = 0;
        if path.is_file() {
            // Single file replacement
            if let Ok(content) = fs::read_to_string(&path).await {
                let (new_content, count) = if let Some(ref re) = regex {
                    let result = re.replace_all(&content, replace.as_str());
                    let count = re.find_iter(&content).count();
                    (result.into_owned(), count)
                } else {
                    let count = content.matches(&search).count();
                    (content.replace(&search, &replace), count)
                };
                
                if count > 0 {
                    if !dry_run {
                        fs::write(&path, &new_content).await?;
                    }
                    
                    xml_results.push_str(&format!(
                        "<file><path>{}</path><replacements>{}</replacements></file>",
                        path.display(),
                        count
                    ));
                    total_replacements += count;
                    files_modified += 1;
                }
            }
        } else {
            // Directory traversal
            for entry in WalkDir::new(&path)
                .follow_links(false)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_file())
            {
                // Check file type filter
                if !file_types.is_empty() {
                    if let Some(ext) = entry.path().extension() {
                        let ext_str = ext.to_string_lossy();
                        if !file_types.iter().any(|ft| ft == &ext_str) {
                            continue;
                        }
                    } else {
                        continue;
                    }
                }
                
                // Skip binary files
                if let Ok(content) = fs::read_to_string(entry.path()).await {
                    // Skip binary files (check for common binary file signatures)
                    let bytes = content.as_bytes();
                    if bytes.len() >= 4 && (
                        (bytes[0] == 0x89 && bytes[1] == 0x50 && bytes[2] == 0x4e && bytes[3] == 0x47) || // PNG
                        (bytes[0] == 0xff && bytes[1] == 0xd8 && bytes[2] == 0xff) || // JPEG
                        (bytes[0] == 0x47 && bytes[1] == 0x49 && bytes[2] == 0x46) // GIF
                    ) {
                        continue;
                    }
                    let (new_content, count) = if let Some(ref re) = regex {
                        let result = re.replace_all(&content, replace.as_str());
                        let count = re.find_iter(&content).count();
                        (result.into_owned(), count)
                    } else {
                        let count = content.matches(&search).count();
                        (content.replace(&search, &replace), count)
                    };
                    if count > 0 {
                        if !dry_run {
                            fs::write(entry.path(), &new_content).await?;
                        }
                        
                        xml_results.push_str(&format!(
                            "<file><path>{}</path><replacements>{}</replacements></file>",
                            entry.path().display(),
                            count
                        ));
                        total_replacements += count;
                        files_modified += 1;
                        // Limit output
                        if files_modified >= 100 {
                            break;
                        }
                    }
                }
            }
        }
        
        xml_results.push_str("</files>");
        let xml_response = format!(
            "<tool_result><success>true</success><search>{}</search><replace>{}</replace><dry_run>{}</dry_run>{}<total_replacements>{}</total_replacements><files_modified>{}</files_modified></tool_result>",
            html_escape::encode_text(&search),
            html_escape::encode_text(&replace),
            dry_run,
            xml_results,
            total_replacements,
            files_modified
        );
        Ok(ToolResult::from_xml(xml_response))
    }
    
    fn required_permissions(&self) -> Vec<Permission> {
        vec![Permission::FileWrite("*".to_string())]
    }
}
