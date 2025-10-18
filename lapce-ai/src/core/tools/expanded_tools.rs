// Expanded tool set for ToolRegistry - P1-7
// Adds 10+ additional tools to reach 20+ total

use async_trait::async_trait;
use serde_json::{json, Value};
use anyhow::Result;

use crate::core::tools::traits::{Tool, ToolContext, ToolResult, ToolOutput, ToolError};

// ============= Git Tools =============

pub struct GitStatusTool;

#[async_trait]
impl Tool for GitStatusTool {
    fn name(&self) -> &'static str { "git_status" }
    fn description(&self) -> &'static str { "Get git repository status" }
    
    async fn execute(&self, _args: Value, context: ToolContext) -> ToolResult {
        let output = std::process::Command::new("git")
            .arg("status")
            .arg("--porcelain")
            .current_dir(&context.workspace)
            .output()
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;
        
        Ok(ToolOutput {
            success: true,
            result: json!({
                "status": String::from_utf8_lossy(&output.stdout).to_string()
            }),
            error: None,
            metadata: Default::default(),
        })
    }
}

pub struct GitDiffTool;

#[async_trait]
impl Tool for GitDiffTool {
    fn name(&self) -> &'static str { "git_diff" }
    fn description(&self) -> &'static str { "Show git diff for files" }
    fn requires_approval(&self) -> bool { false }
    
    async fn execute(&self, args: Value, context: ToolContext) -> ToolResult {
        let file = args["file"].as_str().unwrap_or("");
        
        let output = std::process::Command::new("git")
            .arg("diff")
            .arg(file)
            .current_dir(&context.workspace)
            .output()
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;
        
        Ok(ToolOutput {
            success: true,
            result: json!({
                "diff": String::from_utf8_lossy(&output.stdout).to_string()
            }),
            error: None,
            metadata: Default::default(),
        })
    }
}

// ============= File Analysis Tools =============

pub struct CountLinesTool;

#[async_trait]
impl Tool for CountLinesTool {
    fn name(&self) -> &'static str { "count_lines" }
    fn description(&self) -> &'static str { "Count lines in files" }
    
    async fn execute(&self, args: Value, context: ToolContext) -> ToolResult {
        let path = args["path"].as_str().unwrap_or(".");
        let full_path = context.resolve_path(path);
        
        let content = tokio::fs::read_to_string(&full_path).await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;
        
        let lines = content.lines().count();
        let non_empty = content.lines().filter(|l| !l.trim().is_empty()).count();
        
        Ok(ToolOutput {
            success: true,
            result: json!({
                "total_lines": lines,
                "non_empty_lines": non_empty,
                "empty_lines": lines - non_empty
            }),
            error: None,
            metadata: Default::default(),
        })
    }
}

pub struct FileSizeTool;

#[async_trait]
impl Tool for FileSizeTool {
    fn name(&self) -> &'static str { "file_size" }
    fn description(&self) -> &'static str { "Get file or directory size" }
    
    async fn execute(&self, args: Value, context: ToolContext) -> ToolResult {
        let path = args["path"].as_str().unwrap_or(".");
        let full_path = context.resolve_path(path);
        
        let metadata = tokio::fs::metadata(&full_path).await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;
        
        Ok(ToolOutput {
            success: true,
            result: json!({
                "size_bytes": metadata.len(),
                "size_kb": metadata.len() / 1024,
                "size_mb": metadata.len() / (1024 * 1024),
                "is_file": metadata.is_file(),
                "is_dir": metadata.is_dir()
            }),
            error: None,
            metadata: Default::default(),
        })
    }
}

// ============= System Tools =============

pub struct ProcessListTool;

#[async_trait]
impl Tool for ProcessListTool {
    fn name(&self) -> &'static str { "process_list" }
    fn description(&self) -> &'static str { "List running processes" }
    fn requires_approval(&self) -> bool { true }
    
    async fn execute(&self, _args: Value, _context: ToolContext) -> ToolResult {
        let output = std::process::Command::new("ps")
            .arg("aux")
            .output()
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;
        
        Ok(ToolOutput {
            success: true,
            result: json!({
                "processes": String::from_utf8_lossy(&output.stdout).to_string()
            }),
            error: None,
            metadata: Default::default(),
        })
    }
}

pub struct EnvironmentTool;

#[async_trait]
impl Tool for EnvironmentTool {
    fn name(&self) -> &'static str { "environment" }
    fn description(&self) -> &'static str { "Get environment variables" }
    
    async fn execute(&self, args: Value, _context: ToolContext) -> ToolResult {
        let var_name = args["name"].as_str();
        
        let result = if let Some(name) = var_name {
            json!({
                name: std::env::var(name).unwrap_or_default()
            })
        } else {
            let vars: std::collections::HashMap<String, String> = 
                std::env::vars().collect();
            json!(vars)
        };
        
        Ok(ToolOutput {
            success: true,
            result,
            error: None,
            metadata: Default::default(),
        })
    }
}

// ============= Text Processing Tools =============

pub struct Base64Tool;

#[async_trait]
impl Tool for Base64Tool {
    fn name(&self) -> &'static str { "base64" }
    fn description(&self) -> &'static str { "Encode/decode base64" }
    
    async fn execute(&self, args: Value, _context: ToolContext) -> ToolResult {
        use base64::{Engine as _, engine::general_purpose};
        
        let action = args["action"].as_str().unwrap_or("encode");
        let input = args["input"].as_str().unwrap_or("");
        
        let result = match action {
            "encode" => {
                let encoded = general_purpose::STANDARD.encode(input);
                json!({ "output": encoded })
            }
            "decode" => {
                match general_purpose::STANDARD.decode(input) {
                    Ok(decoded) => json!({ "output": String::from_utf8_lossy(&decoded).to_string() }),
                    Err(e) => json!({ "error": e.to_string() })
                }
            }
            _ => json!({ "error": "Invalid action" })
        };
        
        Ok(ToolOutput {
            success: true,
            result,
            error: None,
            metadata: Default::default(),
        })
    }
}

pub struct JsonFormatTool;

#[async_trait]
impl Tool for JsonFormatTool {
    fn name(&self) -> &'static str { "json_format" }
    fn description(&self) -> &'static str { "Format/validate JSON" }
    
    async fn execute(&self, args: Value, _context: ToolContext) -> ToolResult {
        let input = args["input"].as_str().unwrap_or("{}");
        
        let result = match serde_json::from_str::<Value>(input) {
            Ok(parsed) => {
                let pretty = serde_json::to_string_pretty(&parsed).unwrap();
                json!({
                    "valid": true,
                    "formatted": pretty
                })
            }
            Err(e) => {
                json!({
                    "valid": false,
                    "error": e.to_string()
                })
            }
        };
        
        Ok(ToolOutput {
            success: true,
            result,
            error: None,
            metadata: Default::default(),
        })
    }
}

// ============= Archive Tools =============

pub struct ZipTool;

#[async_trait]
impl Tool for ZipTool {
    fn name(&self) -> &'static str { "zip" }
    fn description(&self) -> &'static str { "Create/extract zip archives" }
    fn requires_approval(&self) -> bool { true }
    
    async fn execute(&self, args: Value, context: ToolContext) -> ToolResult {
        let action = args["action"].as_str().unwrap_or("create");
        let archive = args["archive"].as_str().unwrap_or("archive.zip");
        let files = args["files"].as_array();
        
        let mut cmd = std::process::Command::new("zip");
        
        if action == "create" {
            cmd.arg("-r").arg(archive);
            if let Some(file_list) = files {
                for file in file_list {
                    if let Some(f) = file.as_str() {
                        cmd.arg(f);
                    }
                }
            }
        } else {
            cmd.arg("-x").arg(archive);
        }
        
        cmd.current_dir(&context.workspace);
        
        let output = cmd.output()
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;
        
        Ok(ToolOutput {
            success: output.status.success(),
            result: json!({
                "success": output.status.success(),
                "output": String::from_utf8_lossy(&output.stdout).to_string()
            }),
            error: None,
            metadata: Default::default(),
        })
    }
}

// ============= Network Tools =============

pub struct CurlTool;

#[async_trait]
impl Tool for CurlTool {
    fn name(&self) -> &'static str { "curl" }
    fn description(&self) -> &'static str { "Make HTTP requests" }
    fn requires_approval(&self) -> bool { true }
    
    async fn execute(&self, args: Value, _context: ToolContext) -> ToolResult {
        let url = args["url"].as_str()
            .ok_or_else(|| ToolError::InvalidArguments("Missing URL".to_string()))?;
        
        let method = args["method"].as_str().unwrap_or("GET");
        
        let mut cmd = std::process::Command::new("curl");
        cmd.arg("-X").arg(method)
           .arg("-s")
           .arg(url);
        
        if let Some(headers) = args["headers"].as_object() {
            for (key, val) in headers {
                if let Some(v) = val.as_str() {
                    cmd.arg("-H").arg(format!("{}: {}", key, v));
                }
            }
        }
        
        if let Some(body) = args["body"].as_str() {
            cmd.arg("-d").arg(body);
        }
        
        let output = cmd.output()
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;
        
        Ok(ToolOutput {
            success: output.status.success(),
            result: json!({
                "response": String::from_utf8_lossy(&output.stdout).to_string(),
                "success": output.status.success()
            }),
            error: None,
            metadata: Default::default(),
        })
    }
}

/// Register all expanded tools
pub fn register_expanded_tools(registry: &crate::core::tools::registry::ToolRegistry) -> Result<()> {
    // Git tools
    registry.register(GitStatusTool)?;
    registry.register(GitDiffTool)?;
    
    // File analysis
    registry.register(CountLinesTool)?;
    registry.register(FileSizeTool)?;
    
    // System tools
    registry.register(ProcessListTool)?;
    registry.register(EnvironmentTool)?;
    
    // Text processing
    registry.register(Base64Tool)?;
    registry.register(JsonFormatTool)?;
    
    // Archive
    registry.register(ZipTool)?;
    
    // Network
    registry.register(CurlTool)?;
    
    Ok(())
}
