use crate::mcp_tools::types::FileEntry;
/// COMPLETE 1:1 TypeScript Translation of readFileTool.ts
use std::collections::HashMap;
use async_trait::async_trait;
use serde_json::{json, Value};
use anyhow::{Result, bail};
use tokio::fs;

use crate::mcp_tools::{
    core::{Tool, ToolContext, ToolResult, JsonSchema, ResourceLimits},
    permissions::Permission,
};
#[derive(Debug, Clone)]
struct FileEntry {
    path: String,
    line_ranges: Vec<LineRange>,
}
struct LineRange {
    start: usize,
    end: usize,
pub struct ReadFileTool {
    max_file_size_mb: usize,
    max_image_size_mb: usize,
impl ReadFileTool {
    pub fn new() -> Self {
        Self {
            max_file_size_mb: 10, // DEFAULT_MAX_IMAGE_FILE_SIZE_MB
            max_image_size_mb: 20, // DEFAULT_MAX_TOTAL_IMAGE_SIZE_MB
        }
    }
    
    fn parse_xml_args(&self, xml: &str) -> Result<Vec<FileEntry>> {
        let mut entries = Vec::new();
        
        // Parse XML structure:
        // <args>
        //   <file>
        //     <path>/path/to/file</path>
        //     <line_range>1-50</line_range>
        //   </file>
        // </args>
        if xml.contains("<file>") {
            let file_blocks: Vec<&str> = xml.split("<file>").collect();
            for block in file_blocks.iter().skip(1) {
                if let Some(end_idx) = block.find("</file>") {
                    let file_content = &block[..end_idx];
                    
                    // Extract path
                    let path = if let Some(path_start) = file_content.find("<path>") {
                        let path_start = path_start + 6;
                        if let Some(path_end) = file_content[path_start..].find("</path>") {
                            file_content[path_start..path_start + path_end].to_string()
                        } else {
                            continue;
                        }
                    } else {
                        continue;
                    };
                    // Extract line ranges
                    let mut line_ranges = Vec::new();
                    if let Some(range_start) = file_content.find("<line_range>") {
                        let range_start = range_start + 12;
                        if let Some(range_end) = file_content[range_start..].find("</line_range>") {
                            let range_str = &file_content[range_start..range_start + range_end];
                            if let Some((start, end)) = range_str.split_once('-') {
                                if let (Ok(start), Ok(end)) = (start.parse::<usize>(), end.parse::<usize>()) {
                                    line_ranges.push(LineRange { start, end });
                                }
                            }
                    }
                    entries.push(FileEntry { path, line_ranges });
                }
            }
        } else if xml.contains("<path>") {
            // Legacy single path format
            if let Some(path_start) = xml.find("<path>") {
                let path_start = path_start + 6;
                if let Some(path_end) = xml[path_start..].find("</path>") {
                    let path = xml[path_start..path_start + path_end].to_string();
                    entries.push(FileEntry { path, line_ranges: Vec::new() });
        if entries.is_empty() {
            bail!("No valid file paths found in XML args");
        Ok(entries)
    async fn read_file_content(&self, path: &Path, ranges: &[LineRange]) -> Result<String> {
        // Check if file exists
        if !path.exists() {
            bail!("File not found: {}", path.display());
        // Check if it's a binary file (simplified check)
        let metadata = fs::metadata(path).await?;
        if metadata.len() > (self.max_file_size_mb as u64 * 1024 * 1024) {
            bail!("File too large: {} MB", metadata.len() / (1024 * 1024));
        // Read file content
        let content = fs::read_to_string(path).await?;
        // Apply line ranges if specified
        if !ranges.is_empty() {
            let lines: Vec<&str> = content.lines().collect();
            let mut result = Vec::new();
            
            for range in ranges {
                let start = (range.start - 1).min(lines.len());
                let end = range.end.min(lines.len());
                
                for i in start..end {
                    result.push(format!("{:4} | {}", i + 1, lines[i]));
            Ok(result.join("\n"))
        } else {
            // Add line numbers to full content
            let result: Vec<String> = lines.iter().enumerate()
                .map(|(i, line)| format!("{:4} | {}", i + 1, line))
                .collect();
#[async_trait]
impl Tool for ReadFileTool {
    fn name(&self) -> &str {
        "readFile"
    fn description(&self) -> &str {
    fn parameters(&self) -> Vec<ToolParameter> { vec![] }
        "Read file contents with support for line ranges and multiple files (1:1 TypeScript port)"
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "args": {
                    "type": "string",
                    "description": "XML format with file paths and optional line ranges"
                },
                "path": {
                    "description": "Legacy: single file path"
                "view_range": {
                    "description": "Legacy: line range in format '1-50'"
        })
    async fn validate(&self, args: &Value) -> Result<()> {
        if args.get("args").is_none() && args.get("path").is_none() {
            bail!("Missing required parameter: args or path");
        Ok(())
    async fn execute(&self, args: Value, context: ToolContext) -> Result<ToolResult> {
        // Parse file entries from args
        let files_to_read = if let Some(xml_str) = args.get("args").and_then(|a| a.as_str()) {
            self.parse_xml_args(xml_str)?
        } else if let Some(path) = args.get("path").and_then(|p| p.as_str()) {
            // Legacy support
            let mut ranges = Vec::new();
            if let Some(range_str) = args.get("view_range").and_then(|r| r.as_str()) {
                if let Some((start, end)) = range_str.split_once('-') {
                    if let (Ok(start), Ok(end)) = (start.parse::<usize>(), end.parse::<usize>()) {
                        ranges.push(LineRange { start, end });
            vec![FileEntry { path: path.to_string(), line_ranges: ranges }]
        };
        // Process each file
        let mut results = Vec::new();
        let mut errors = Vec::new();
        for entry in files_to_read {
            let full_path = context.workspace.join(&entry.path);
            match self.read_file_content(&full_path, &entry.line_ranges).await {
                Ok(content) => {
                    results.push(json!({
                        "path": entry.path,
                        "content": content,
                        "exists": true
                    }));
                Err(e) => {
                    errors.push(json!({
                        "error": e.to_string()
        // Format response in XML like TypeScript version
        let mut xml_response = String::from("<files>\n");
        for result in results {
            xml_response.push_str(&format!(
                "<file>\n<path>{}</path>\n<content>\n{}\n</content>\n</file>\n",
                result["path"].as_str().unwrap_or(""),
                result["content"].as_str().unwrap_or("")
            ));
        for error in &errors {
                "<error>Failed to read {}: {}</error>\n",
                error["path"].as_str().unwrap_or(""),
                error["error"].as_str().unwrap_or("")
        xml_response.push_str("</files>");
        Ok(ToolResult {
            success: errors.is_empty(),
            data: json!({
                "files": results,
                "errors": errors,
                "xml": xml_response
            }),
    fn required_permissions(&self) -> Vec<Permission> {
        vec![Permission::WorkspaceRead]
            max_file_size: self.max_file_size_mb * 1024 * 1024,
