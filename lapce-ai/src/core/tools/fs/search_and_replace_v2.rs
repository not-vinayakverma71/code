// Search and Replace Tool v2 - Hardened with line range support
// Part of Core FS tools hardening - pre-IPC TODO

use crate::core::tools::traits::{Tool, ToolContext, ToolOutput, ToolResult, ToolError};
use crate::core::tools::xml_util::XmlParser;
use serde_json::{json, Value};
use std::fs;
use regex::{Regex, RegexBuilder};
use super::utils::{
    self, FileInfo, SymlinkPolicy, MAX_FILE_SIZE, MAX_WRITE_SIZE,
    get_file_info, read_file_safe, write_file_safe,
    apply_to_line_range, preserve_line_endings
};
use super::ensure_workspace_path;
use async_trait::async_trait;

pub struct SearchAndReplaceToolV2;

#[async_trait]
impl Tool for SearchAndReplaceToolV2 {
    fn name(&self) -> &'static str {
        "searchAndReplace"
    }
    
    fn description(&self) -> &'static str {
        "Search and replace text in a file with enhanced line range support and encoding preservation (requires approval)"
    }
    
    fn requires_approval(&self) -> bool {
        true
    }
    
    async fn execute(&self, args: Value, context: ToolContext) -> ToolResult {
        // Parse XML arguments
        let parser = XmlParser::new();
        let parsed = parser.parse(args.as_str().ok_or_else(|| {
            ToolError::InvalidArguments("Expected XML string".to_string())
        })?).map_err(|e| ToolError::InvalidArguments(e.to_string()))?;
        
        // Extract tool data
        let tool_data = super::extract_tool_data(&parsed);
        
        let path = tool_data["path"].as_str()
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'path' argument".to_string()))?;
            
        let search = tool_data["search"].as_str()
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'search' argument".to_string()))?;
            
        let replace = tool_data["replace"].as_str()
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'replace' argument".to_string()))?;
            
        let mode = tool_data.get("mode")
            .and_then(|v| v.as_str())
            .unwrap_or("literal");
            
        let multiline = tool_data.get("multiline")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
            
        let preview_only = tool_data.get("preview")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
            
        // New v2 options
        let case_insensitive = tool_data.get("caseInsensitive")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
            
        let whole_word = tool_data.get("wholeWord")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
            
        let line_start = tool_data.get("lineStart")
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse::<usize>().ok());
            
        let line_end = tool_data.get("lineEnd")
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse::<usize>().ok());
            
        let preserve_encoding = tool_data.get("preserveEncoding")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
            
        let max_replacements = tool_data.get("maxReplacements")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize);
            
        let backup_if_changed = tool_data.get("backupIfChanged")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        
        // Validate line range if provided
        if let (Some(start), Some(end)) = (line_start, line_end) {
            if start > end {
                return Err(ToolError::InvalidArguments(format!(
                    "Invalid line range: start ({}) > end ({})",
                    start, end
                )));
            }
        } else if line_start.is_some() || line_end.is_some() {
            return Err(ToolError::InvalidArguments(
                "Both lineStart and lineEnd must be specified for line range".to_string()
            ));
        }
        
        // Resolve path
        let file_path = context.resolve_path(path);
        
        // Check .rooignore
        if !context.is_path_allowed(&file_path) {
            return Err(ToolError::RooIgnoreBlocked(format!(
                "Path '{}' is blocked by .rooignore",
                file_path.display()
            )));
        }
        
        // Check permissions (skip for preview-only)
        if !preview_only && !context.can_write_file(&file_path).await {
            return Err(ToolError::PermissionDenied(format!(
                "Permission denied to write file: {}",
                file_path.display()
            )));
        }
        
        // Ensure path is within workspace
        let safe_path = ensure_workspace_path(&context.workspace, &file_path)
            .map_err(|e| ToolError::PermissionDenied(e))?;
        
        // File must exist
        if !safe_path.exists() {
            return Err(ToolError::InvalidArguments(format!(
                "File does not exist: {}",
                path
            )));
        }
        
        // Check for symlinks
        if !preview_only && utils::is_symlink(&safe_path).unwrap_or(false) {
            return Err(ToolError::Other(format!(
                "Cannot modify symlink: {}. Please modify the target file directly.",
                path
            )));
        }
        
        // Read file content with safety checks
        let (content, file_info) = read_file_safe(&safe_path, MAX_FILE_SIZE)
            .map_err(|e| ToolError::Io(e))?;
        
        // Build search pattern based on mode and options
        let search_pattern = build_search_pattern(search, mode, case_insensitive, multiline, whole_word)?;
        
        // Perform replacement
        let (new_content, replacements) = if let (Some(start), Some(end)) = (line_start, line_end) {
            // Apply to specific line range
            perform_line_range_replacement(
                &content,
                start,
                end,
                &search_pattern,
                replace,
                mode,
                max_replacements
            )?
        } else {
            // Apply to entire file
            perform_full_replacement(
                &content,
                &search_pattern,
                replace,
                mode,
                max_replacements
            )?
        };
        
        // Preserve original line endings if requested
        let final_content = if preserve_encoding {
            preserve_line_endings(&content, &new_content)
        } else {
            new_content
        };
        
        // Build detailed change summary
        let change_summary = build_change_summary(
            &content,
            &final_content,
            replacements.clone(),
            line_start,
            line_end
        );
        
        // Preview mode: just return the diff
        if preview_only {
            return Ok(ToolOutput::success(json!({
                "file": path,
                "mode": mode,
                "matches_found": replacements.len(),
                "preview": true,
                "lineRange": if line_start.is_some() {
                    Some(json!({ "start": line_start, "end": line_end }))
                } else {
                    None
                },
                "changes": change_summary,
                "original_size": content.len(),
                "new_size": final_content.len(),
            })));
        }
        
        // Request approval if required
        if context.require_approval && !context.dry_run {
            return Err(ToolError::ApprovalRequired(format!(
                "Approval required to replace {} occurrence(s) in file: {}",
                replacements.len(), path
            )));
        }
        
        // Dry-run mode
        if context.dry_run {
            return Ok(ToolOutput::success(json!({
                "file": path,
                "mode": mode,
                "matches_found": replacements.len(),
                "dry_run": true,
                "would_modify": !replacements.is_empty(),
                "lineRange": if line_start.is_some() {
                    Some(json!({ "start": line_start, "end": line_end }))
                } else {
                    None
                },
            })));
        }
        
        // Only write if there were changes
        if !replacements.is_empty() {
            // Create backup if requested
            if backup_if_changed {
                let backup_path = format!("{}.backup", safe_path.display());
                fs::copy(&safe_path, &backup_path)
                    .map_err(|e| ToolError::Io(e))?;
            }
            
            // Write the file with safety checks
            write_file_safe(&safe_path, &final_content, preserve_encoding, MAX_WRITE_SIZE)
                .map_err(|e| ToolError::Io(e))?;
        }
        
        Ok(ToolOutput::success(json!({
            "file": path,
            "mode": mode,
            "replacements_made": replacements.len(),
            "lineRange": if line_start.is_some() {
                Some(json!({ "start": line_start, "end": line_end }))
            } else {
                None
            },
            "original_size": content.len(),
            "new_size": final_content.len(),
            "backup": if backup_if_changed && !replacements.is_empty() {
                Some(format!("{}.backup", path))
            } else {
                None
            },
            "details": replacements,
        })))
    }
}

#[derive(Debug)]
enum SearchPattern {
    Literal(String, bool), // (pattern, case_insensitive)
    Regex(Regex),
    WholeWord(String, bool), // (word, case_insensitive)
}

fn build_search_pattern(
    search: &str,
    mode: &str,
    case_insensitive: bool,
    multiline: bool,
    whole_word: bool,
) -> Result<SearchPattern, ToolError> {
    match mode {
        "regex" => {
            let mut builder = RegexBuilder::new(search);
            builder.case_insensitive(case_insensitive);
            builder.multi_line(multiline);
            
            let regex = builder.build()
                .map_err(|e| ToolError::InvalidArguments(format!("Invalid regex: {}", e)))?;
            
            Ok(SearchPattern::Regex(regex))
        }
        "literal" => {
            if whole_word {
                Ok(SearchPattern::WholeWord(search.to_string(), case_insensitive))
            } else {
                Ok(SearchPattern::Literal(search.to_string(), case_insensitive))
            }
        }
        _ => Err(ToolError::InvalidArguments(format!(
            "Invalid mode '{}'. Must be 'literal' or 'regex'",
            mode
        )))
    }
}

fn perform_full_replacement(
    content: &str,
    pattern: &SearchPattern,
    replace: &str,
    mode: &str,
    max_replacements: Option<usize>,
) -> Result<(String, Vec<Value>), ToolError> {
    let mut replacements = Vec::new();
    
    let new_content = match pattern {
        SearchPattern::Regex(regex) => {
            // Collect match info before replacement
            for (idx, mat) in regex.find_iter(content).enumerate() {
                if let Some(max) = max_replacements {
                    if idx >= max {
                        break;
                    }
                }
                
                let line_num = content[..mat.start()]
                    .chars()
                    .filter(|&c| c == '\n')
                    .count() + 1;
                
                replacements.push(json!({
                    "line": line_num,
                    "offset": mat.start(),
                    "matched": mat.as_str(),
                    "replacement": replace,
                }));
            }
            
            if let Some(max) = max_replacements {
                regex.replacen(content, max, replace).to_string()
            } else {
                regex.replace_all(content, replace).to_string()
            }
        }
        SearchPattern::Literal(search, case_insensitive) => {
            let matches = find_literal_matches(content, search, *case_insensitive);
            
            let mut result = String::new();
            let mut last_end = 0;
            
            for (idx, (start, end, matched)) in matches.iter().enumerate() {
                if let Some(max) = max_replacements {
                    if idx >= max {
                        result.push_str(&content[last_end..]);
                        break;
                    }
                }
                
                let line_num = content[..*start]
                    .chars()
                    .filter(|&c| c == '\n')
                    .count() + 1;
                
                replacements.push(json!({
                    "line": line_num,
                    "offset": start,
                    "matched": matched,
                    "replacement": replace,
                }));
                
                result.push_str(&content[last_end..*start]);
                result.push_str(replace);
                last_end = *end;
            }
            
            if last_end < content.len() {
                result.push_str(&content[last_end..]);
            }
            
            result
        }
        SearchPattern::WholeWord(word, case_insensitive) => {
            let pattern = format!(r"\b{}\b", regex::escape(word));
            let mut builder = RegexBuilder::new(&pattern);
            builder.case_insensitive(*case_insensitive);
            
            let regex = builder.build()
                .map_err(|e| ToolError::Other(format!("Failed to build word regex: {}", e)))?;
            
            // Collect match info
            for (idx, mat) in regex.find_iter(content).enumerate() {
                if let Some(max) = max_replacements {
                    if idx >= max {
                        break;
                    }
                }
                
                let line_num = content[..mat.start()]
                    .chars()
                    .filter(|&c| c == '\n')
                    .count() + 1;
                
                replacements.push(json!({
                    "line": line_num,
                    "offset": mat.start(),
                    "matched": mat.as_str(),
                    "replacement": replace,
                }));
            }
            
            if let Some(max) = max_replacements {
                regex.replacen(content, max, replace).to_string()
            } else {
                regex.replace_all(content, replace).to_string()
            }
        }
    };
    
    Ok((new_content, replacements))
}

fn perform_line_range_replacement(
    content: &str,
    start: usize,
    end: usize,
    pattern: &SearchPattern,
    replace: &str,
    mode: &str,
    max_replacements: Option<usize>,
) -> Result<(String, Vec<Value>), ToolError> {
    let mut replacements = Vec::new();
    let mut total_replacements = 0;
    
    let new_content = apply_to_line_range(content, start, end, |line| {
        if let Some(max) = max_replacements {
            if total_replacements >= max {
                return line.to_string();
            }
        }
        
        let (new_line, line_replacements) = match pattern {
            SearchPattern::Regex(regex) => {
                let matches_in_line = regex.find_iter(line).count();
                if matches_in_line > 0 {
                    let replacements_to_make = if let Some(max) = max_replacements {
                        (max - total_replacements).min(matches_in_line)
                    } else {
                        matches_in_line
                    };
                    
                    total_replacements += replacements_to_make;
                    
                    if replacements_to_make < matches_in_line {
                        (regex.replacen(line, replacements_to_make, replace).to_string(), replacements_to_make)
                    } else {
                        (regex.replace_all(line, replace).to_string(), matches_in_line)
                    }
                } else {
                    (line.to_string(), 0)
                }
            }
            SearchPattern::Literal(search, case_insensitive) => {
                let matches = find_literal_matches(line, search, *case_insensitive);
                if !matches.is_empty() {
                    let replacements_to_make = if let Some(max) = max_replacements {
                        (max - total_replacements).min(matches.len())
                    } else {
                        matches.len()
                    };
                    
                    total_replacements += replacements_to_make;
                    
                    let mut result = String::new();
                    let mut last_end = 0;
                    
                    for (idx, (start, end, _)) in matches.iter().enumerate() {
                        if idx >= replacements_to_make {
                            result.push_str(&line[last_end..]);
                            break;
                        }
                        result.push_str(&line[last_end..*start]);
                        result.push_str(replace);
                        last_end = *end;
                    }
                    
                    if last_end < line.len() {
                        result.push_str(&line[last_end..]);
                    }
                    
                    (result, replacements_to_make)
                } else {
                    (line.to_string(), 0)
                }
            }
            SearchPattern::WholeWord(word, case_insensitive) => {
                let pattern = format!(r"\b{}\b", regex::escape(word));
                let mut builder = RegexBuilder::new(&pattern);
                builder.case_insensitive(*case_insensitive);
                
                if let Ok(regex) = builder.build() {
                    let matches_in_line = regex.find_iter(line).count();
                    if matches_in_line > 0 {
                        let replacements_to_make = if let Some(max) = max_replacements {
                            (max - total_replacements).min(matches_in_line)
                        } else {
                            matches_in_line
                        };
                        
                        total_replacements += replacements_to_make;
                        
                        if replacements_to_make < matches_in_line {
                            (regex.replacen(line, replacements_to_make, replace).to_string(), replacements_to_make)
                        } else {
                            (regex.replace_all(line, replace).to_string(), matches_in_line)
                        }
                    } else {
                        (line.to_string(), 0)
                    }
                } else {
                    (line.to_string(), 0)
                }
            }
        };
        
        // Track replacement details
        for _ in 0..line_replacements {
            replacements.push(json!({
                "lineRange": true,
                "replacement": replace,
            }));
        }
        
        new_line
    });
    
    Ok((new_content, replacements))
}

fn find_literal_matches(content: &str, search: &str, case_insensitive: bool) -> Vec<(usize, usize, String)> {
    let mut matches = Vec::new();
    
    let (search_text, content_text) = if case_insensitive {
        (search.to_lowercase(), content.to_lowercase())
    } else {
        (search.to_string(), content.to_string())
    };
    
    let mut start = 0;
    while let Some(pos) = content_text[start..].find(&search_text) {
        let match_start = start + pos;
        let match_end = match_start + search.len();
        let matched = content[match_start..match_end].to_string();
        matches.push((match_start, match_end, matched));
        start = match_end;
    }
    
    matches
}

fn build_change_summary(
    original: &str,
    modified: &str,
    replacements: Vec<Value>,
    line_start: Option<usize>,
    line_end: Option<usize>,
) -> Value {
    let original_lines = original.lines().count();
    let modified_lines = modified.lines().count();
    
    json!({
        "replacements": replacements.len(),
        "originalLines": original_lines,
        "modifiedLines": modified_lines,
        "sizeDelta": (modified.len() as i64) - (original.len() as i64),
        "lineRange": if line_start.is_some() {
            Some(json!({
                "start": line_start,
                "end": line_end,
            }))
        } else {
            None
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;
    
    #[tokio::test]
    async fn test_line_range_replacement() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("lines.txt");
        
        fs::write(&file_path, "foo line 1\nbar line 2\nfoo line 3\nbar line 4\nfoo line 5").unwrap();
        
        let tool = SearchAndReplaceToolV2;
        let mut context = ToolContext::new(temp_dir.path().to_path_buf(), "test_user".to_string());
        context.permissions.file_write = true;
        context.require_approval = false;
        
        // Replace only in lines 2-4
        let args = json!(format!(r#"
            <tool>
                <path>lines.txt</path>
                <search>foo</search>
                <replace>replaced</replace>
                <mode>literal</mode>
                <lineStart>2</lineStart>
                <lineEnd>4</lineEnd>
            </tool>
        "#));
        
        let result = tool.execute(args, context).await.unwrap();
        assert!(result.success);
        assert_eq!(result.result["replacements_made"], 1); // Only line 3 has "foo"
        
        let content = fs::read_to_string(&file_path).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        
        assert_eq!(lines[0], "foo line 1"); // Unchanged (outside range)
        assert_eq!(lines[2], "replaced line 3"); // Changed (in range)
        assert_eq!(lines[4], "foo line 5"); // Unchanged (outside range)
    }
    
    #[tokio::test]
    async fn test_case_insensitive_search() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("case.txt");
        
        fs::write(&file_path, "Hello HELLO hello HeLLo").unwrap();
        
        let tool = SearchAndReplaceToolV2;
        let mut context = ToolContext::new(temp_dir.path().to_path_buf(), "test_user".to_string());
        context.permissions.file_write = true;
        context.require_approval = false;
        
        let args = json!(format!(r#"
            <tool>
                <path>case.txt</path>
                <search>hello</search>
                <replace>hi</replace>
                <mode>literal</mode>
                <caseInsensitive>true</caseInsensitive>
            </tool>
        "#));
        
        let result = tool.execute(args, context).await.unwrap();
        assert!(result.success);
        assert_eq!(result.result["replacements_made"], 4);
        
        let content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "hi hi hi hi");
    }
    
    #[tokio::test]
    async fn test_whole_word_matching() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("word.txt");
        
        fs::write(&file_path, "test testing tested tester test").unwrap();
        
        let tool = SearchAndReplaceToolV2;
        let mut context = ToolContext::new(temp_dir.path().to_path_buf(), "test_user".to_string());
        context.permissions.file_write = true;
        context.require_approval = false;
        
        let args = json!(format!(r#"
            <tool>
                <path>word.txt</path>
                <search>test</search>
                <replace>check</replace>
                <mode>literal</mode>
                <wholeWord>true</wholeWord>
            </tool>
        "#));
        
        let result = tool.execute(args, context).await.unwrap();
        assert!(result.success);
        assert_eq!(result.result["replacements_made"], 2); // Only whole word "test"
        
        let content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "check testing tested tester check");
    }
    
    #[tokio::test]
    async fn test_max_replacements() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("max.txt");
        
        fs::write(&file_path, "a a a a a").unwrap();
        
        let tool = SearchAndReplaceToolV2;
        let mut context = ToolContext::new(temp_dir.path().to_path_buf(), "test_user".to_string());
        context.permissions.file_write = true;
        context.require_approval = false;
        
        let args = json!(format!(r#"
            <tool>
                <path>max.txt</path>
                <search>a</search>
                <replace>b</replace>
                <mode>literal</mode>
                <maxReplacements>3</maxReplacements>
            </tool>
        "#));
        
        let result = tool.execute(args, context).await.unwrap();
        assert!(result.success);
        assert_eq!(result.result["replacements_made"], 3);
        
        let content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "b b b a a");
    }
    
    #[tokio::test]
    async fn test_backup_creation() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("backup.txt");
        
        fs::write(&file_path, "original content").unwrap();
        
        let tool = SearchAndReplaceToolV2;
        let mut context = ToolContext::new(temp_dir.path().to_path_buf(), "test_user".to_string());
        context.permissions.file_write = true;
        context.require_approval = false;
        
        let args = json!(format!(r#"
            <tool>
                <path>backup.txt</path>
                <search>original</search>
                <replace>modified</replace>
                <mode>literal</mode>
                <backupIfChanged>true</backupIfChanged>
            </tool>
        "#));
        
        let result = tool.execute(args, context).await.unwrap();
        assert!(result.success);
        assert!(result.result["backup"].is_string());
        
        // Check backup exists
        let backup_path = temp_dir.path().join("backup.txt.backup");
        assert!(backup_path.exists());
        assert_eq!(fs::read_to_string(&backup_path).unwrap(), "original content");
        
        // Check modified file
        assert_eq!(fs::read_to_string(&file_path).unwrap(), "modified content");
    }
    
    #[tokio::test]
    async fn test_symlink_rejection() {
        let temp_dir = TempDir::new().unwrap();
        let target_path = temp_dir.path().join("target.txt");
        let link_path = temp_dir.path().join("link.txt");
        
        fs::write(&target_path, "content").unwrap();
        
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(&target_path, &link_path).unwrap();
            
            let tool = SearchAndReplaceToolV2;
            let mut context = ToolContext::new(temp_dir.path().to_path_buf(), "test_user".to_string());
            context.permissions.file_write = true;
            context.require_approval = false;
            
            let args = json!(format!(r#"
                <tool>
                    <path>link.txt</path>
                    <search>content</search>
                    <replace>modified</replace>
                    <mode>literal</mode>
                </tool>
            "#));
            
            let result = tool.execute(args, context).await;
            assert!(result.is_err());
            
            if let Err(ToolError::Other(msg)) = result {
                assert!(msg.contains("Cannot modify symlink"));
            } else {
                panic!("Expected symlink error");
            }
        }
    }
}
