// SearchFiles tool implementation - P0-4

use crate::core::tools::traits::{Tool, ToolContext, ToolResult, ToolOutput, ToolError};
use crate::core::tools::xml_util::XmlParser;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::Path;
use globset::{Glob, GlobSetBuilder};
use regex::Regex;

pub struct SearchFilesTool;

#[async_trait]
impl Tool for SearchFilesTool {
    fn name(&self) -> &'static str {
        "searchFiles"
    }
    
    fn description(&self) -> &'static str {
        "Search for text patterns in files with glob filtering"
    }
    
    async fn execute(&self, args: Value, context: ToolContext) -> ToolResult {
        // Parse XML arguments
        let parser = XmlParser::new();
        let parsed = parser.parse(args.as_str().ok_or_else(|| {
            ToolError::InvalidArguments("Expected XML string".to_string())
        })?).map_err(|e| ToolError::InvalidArguments(e.to_string()))?;
        
        // Extract arguments - handle both flat and nested structures
        let tool_data = super::extract_tool_data(&parsed);
        
        let path = tool_data.get("path")
            .and_then(|v| v.as_str())
            .unwrap_or(".");
            
        let query = tool_data["query"].as_str()
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'query' argument".to_string()))?;
        
        let file_pattern = tool_data.get("filePattern")
            .and_then(|v| v.as_str());
            
        let is_regex = tool_data.get("isRegex")
            .and_then(|v| v.as_str())
            .map(|s| s == "true")
            .unwrap_or(false);
            
        let case_sensitive = tool_data.get("caseSensitive")
            .and_then(|v| v.as_str())
            .map(|s| s == "true")
            .unwrap_or(true);
        
        // Resolve path
        let search_path = context.resolve_path(path);
        
        // Check .rooignore
        if !context.is_path_allowed(&search_path) {
            return Err(ToolError::RooIgnoreBlocked(format!(
                "Path '{}' is blocked by .rooignore",
                path
            )));
        }
        
        // Ensure path is within workspace
        let safe_path = super::ensure_workspace_path(&context.workspace, &search_path)
            .map_err(|e| ToolError::PermissionDenied(e))?;
        
        // Build file pattern matcher
        let glob_matcher = if let Some(pat) = file_pattern {
            let glob = Glob::new(pat)
                .map_err(|e| ToolError::InvalidArguments(format!("Invalid pattern: {}", e)))?;
            let mut builder = GlobSetBuilder::new();
            builder.add(glob);
            Some(builder.build().map_err(|e| ToolError::InvalidArguments(e.to_string()))?)
        } else {
            None
        };
        
        // Build search pattern
        let search_pattern = if is_regex {
            let flags = if case_sensitive { "" } else { "(?i)" };
            Regex::new(&format!("{}{}", flags, query))
                .map_err(|e| ToolError::InvalidArguments(format!("Invalid regex: {}", e)))?
        } else {
            let escaped = regex::escape(query);
            let flags = if case_sensitive { "" } else { "(?i)" };
            Regex::new(&format!("{}{}", flags, escaped))
                .map_err(|e| ToolError::InvalidArguments(format!("Invalid pattern: {}", e)))?
        };
        
        // Search files
        let mut matches = Vec::new();
        if safe_path.is_file() {
            search_file(&safe_path, &search_pattern, &context, &mut matches)?;
        } else {
            search_directory(&safe_path, &search_pattern, &glob_matcher, &context, &mut matches)?;
        }
        
        // Sort matches for consistent output
        matches.sort_by(|a, b| {
            a["file"].as_str().cmp(&b["file"].as_str())
                .then(a["line"].as_u64().cmp(&b["line"].as_u64()))
        });
        
        Ok(ToolOutput::success(json!({
            "query": query,
            "path": path,
            "matches": matches,
            "totalMatches": matches.len(),
        })))
    }
}

fn search_directory(
    dir: &Path,
    pattern: &Regex,
    glob_matcher: &Option<globset::GlobSet>,
    context: &ToolContext,
    matches: &mut Vec<Value>,
) -> Result<(), ToolError> {
    let entries = fs::read_dir(dir).map_err(|e| ToolError::Io(e))?;
    
    for entry in entries {
        let entry = entry.map_err(|e| ToolError::Io(e))?;
        let path = entry.path();
        
        // Check .rooignore
        if !context.is_path_allowed(&path) {
            continue;
        }
        
        let metadata = entry.metadata().map_err(|e| ToolError::Io(e))?;
        
        if metadata.is_file() {
            // Check glob pattern if provided
            if let Some(ref matcher) = glob_matcher {
                let rel_path = path.strip_prefix(&context.workspace)
                    .unwrap_or(&path)
                    .to_string_lossy();
                if !matcher.is_match(&*rel_path) {
                    continue;
                }
            }
            
            // Skip binary files
            if !super::is_binary_file(&path) {
                search_file(&path, pattern, context, matches)?;
            }
        } else if metadata.is_dir() {
            // Recurse into subdirectories
            search_directory(&path, pattern, glob_matcher, context, matches)?;
        }
    }
    
    Ok(())
}

fn search_file(
    path: &Path,
    pattern: &Regex,
    context: &ToolContext,
    matches: &mut Vec<Value>,
) -> Result<(), ToolError> {
    let file = fs::File::open(path).map_err(|e| ToolError::Io(e))?;
    let reader = BufReader::new(file);
    
    let rel_path = path.strip_prefix(&context.workspace)
        .unwrap_or(path)
        .to_string_lossy()
        .to_string();
    
    for (line_num, line) in reader.lines().enumerate() {
        let line = line.map_err(|e| ToolError::Io(e))?;
        
        if let Some(mat) = pattern.find(&line) {
            // Extract context around match
            let start = mat.start().saturating_sub(20);
            let end = (mat.end() + 20).min(line.len());
            let context_str = &line[start..end];
            
            matches.push(json!({
                "file": rel_path,
                "line": line_num + 1,
                "column": mat.start() + 1,
                "match": mat.as_str(),
                "context": context_str,
            }));
        }
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs::{self, File};
    use std::io::Write;
    
    #[tokio::test]
    async fn test_search_files_basic() {
        let temp_dir = TempDir::new().unwrap();
        
        // Create test files
        let mut file1 = File::create(temp_dir.path().join("file1.txt")).unwrap();
        writeln!(file1, "Hello world").unwrap();
        writeln!(file1, "This is a test").unwrap();
        writeln!(file1, "Hello again").unwrap();
        
        let mut file2 = File::create(temp_dir.path().join("file2.txt")).unwrap();
        writeln!(file2, "Another file").unwrap();
        writeln!(file2, "With hello in it").unwrap();
        
        let tool = SearchFilesTool;
        let context = ToolContext::new(temp_dir.path().to_path_buf(), "test_user".to_string());
        
        let args = json!(r#"
            <tool>
                <path>.</path>
                <query>hello</query>
                <caseSensitive>false</caseSensitive>
            </tool>
        "#);
        
        let result = tool.execute(args, context).await.unwrap();
        assert!(result.success);
        
        let matches = result.result["matches"].as_array().unwrap();
        assert_eq!(matches.len(), 3); // "Hello" twice in file1, "hello" once in file2
    }
    
    #[tokio::test]
    async fn test_search_files_regex() {
        let temp_dir = TempDir::new().unwrap();
        
        let mut file = File::create(temp_dir.path().join("test.txt")).unwrap();
        writeln!(file, "test123").unwrap();
        writeln!(file, "test456").unwrap();
        writeln!(file, "no match").unwrap();
        writeln!(file, "test789").unwrap();
        
        let tool = SearchFilesTool;
        let context = ToolContext::new(temp_dir.path().to_path_buf(), "test_user".to_string());
        
        let args = json!(r#"
            <tool>
                <path>.</path>
                <query>test\d+</query>
                <isRegex>true</isRegex>
            </tool>
        "#);
        
        let result = tool.execute(args, context).await.unwrap();
        assert!(result.success);
        
        let matches = result.result["matches"].as_array().unwrap();
        assert_eq!(matches.len(), 3); // test123, test456, test789
    }
    
    #[tokio::test]
    async fn test_search_with_file_pattern() {
        let temp_dir = TempDir::new().unwrap();
        
        // Create files with different extensions
        let mut rs_file = File::create(temp_dir.path().join("code.rs")).unwrap();
        writeln!(rs_file, "fn main() {{ println!(\"target\"); }}").unwrap();
        
        let mut txt_file = File::create(temp_dir.path().join("notes.txt")).unwrap();
        writeln!(txt_file, "target found here").unwrap();
        
        let mut md_file = File::create(temp_dir.path().join("readme.md")).unwrap();
        writeln!(md_file, "# Target Documentation").unwrap();
        
        let tool = SearchFilesTool;
        let context = ToolContext::new(temp_dir.path().to_path_buf(), "test_user".to_string());
        
        let args = json!(r#"
            <tool>
                <path>.</path>
                <query>target</query>
                <filePattern>*.rs</filePattern>
                <caseSensitive>false</caseSensitive>
            </tool>
        "#);
        
        let result = tool.execute(args, context).await.unwrap();
        assert!(result.success);
        
        let matches = result.result["matches"].as_array().unwrap();
        assert_eq!(matches.len(), 1); // Only in .rs file
        assert!(matches[0]["file"].as_str().unwrap().ends_with(".rs"));
    }
}
