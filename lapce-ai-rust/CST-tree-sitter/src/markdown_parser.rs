//! EXACT MARKDOWN PARSER - 1:1 Translation from markdownParser.ts

use regex::Regex;
use std::collections::HashMap;

/// Mock node to mimic tree-sitter node structure
#[derive(Debug, Clone)]
pub struct MockNode {
    pub start_row: usize,
    pub end_row: usize,
    pub text: String,
}

/// Mock capture to mimic tree-sitter capture structure
#[derive(Debug, Clone)]
pub struct MockCapture {
    pub node: MockNode,
    pub name: String,
    pub pattern_index: usize,
}

/// Parse markdown file and extract headers with section ranges
/// This is EXACT translation from Codex markdownParser.ts
pub fn parse_markdown(content: &str) -> Vec<MockCapture> {
    if content.trim().is_empty() {
        return Vec::new();
    }
    
    let lines: Vec<&str> = content.lines().collect();
    let mut captures = Vec::new();
    
    // Regular expressions for different header types
    let atx_header_regex = Regex::new(r"^(#{1,6})\s+(.+)$").unwrap();
    let setext_h1_regex = Regex::new(r"^={3,}\s*$").unwrap();
    let setext_h2_regex = Regex::new(r"^-{3,}\s*$").unwrap();
    let valid_setext_text_regex = Regex::new(r"^\s*[^#<>!\[\]`\t]+[^\n]$").unwrap();
    
    // Find all headers in the document
    for i in 0..lines.len() {
        let line = lines[i];
        
        // Check for ATX headers (# Header)
        if let Some(atx_match) = atx_header_regex.captures(line) {
            let level = atx_match.get(1).unwrap().as_str().len();
            let text = atx_match.get(2).unwrap().as_str().trim().to_string();
            
            // Create a mock node for this header
            let node = MockNode {
                start_row: i,
                end_row: i,
                text: text.clone(),
            };
            
            // Create a mock capture for this header
            captures.push(MockCapture {
                node: node.clone(),
                name: format!("name.definition.header.h{}", level),
                pattern_index: 0,
            });
            
            // Also create a definition capture
            captures.push(MockCapture {
                node,
                name: format!("definition.header.h{}", level),
                pattern_index: 0,
            });
            
            continue;
        }
        
        // Check for setext headers (underlined headers)
        if i > 0 {
            // Check for H1 (======)
            if setext_h1_regex.is_match(line) && valid_setext_text_regex.is_match(lines[i - 1]) {
                let text = lines[i - 1].trim().to_string();
                
                let node = MockNode {
                    start_row: i - 1,
                    end_row: i,
                    text: text.clone(),
                };
                
                captures.push(MockCapture {
                    node: node.clone(),
                    name: "name.definition.header.h1".to_string(),
                    pattern_index: 0,
                });
                
                captures.push(MockCapture {
                    node,
                    name: "definition.header.h1".to_string(),
                    pattern_index: 0,
                });
                
                continue;
            }
            
            // Check for H2 (------)
            if setext_h2_regex.is_match(line) && valid_setext_text_regex.is_match(lines[i - 1]) {
                let text = lines[i - 1].trim().to_string();
                
                let node = MockNode {
                    start_row: i - 1,
                    end_row: i,
                    text: text.clone(),
                };
                
                captures.push(MockCapture {
                    node: node.clone(),
                    name: "name.definition.header.h2".to_string(),
                    pattern_index: 0,
                });
                
                captures.push(MockCapture {
                    node,
                    name: "definition.header.h2".to_string(),
                    pattern_index: 0,
                });
                
                continue;
            }
        }
    }
    
    // Calculate section ranges
    // Sort captures by their start position
    captures.sort_by_key(|c| c.node.start_row);
    
    // Group captures by header (name and definition pairs)
    let mut header_captures: Vec<Vec<MockCapture>> = Vec::new();
    let mut i = 0;
    while i < captures.len() {
        if i + 1 < captures.len() {
            header_captures.push(vec![captures[i].clone(), captures[i + 1].clone()]);
            i += 2;
        } else {
            header_captures.push(vec![captures[i].clone()]);
            i += 1;
        }
    }
    
    // Update end positions for section ranges
    for i in 0..header_captures.len() {
        if i < header_captures.len() - 1 {
            // End position is the start of the next header minus 1
            let next_header_start_row = header_captures[i + 1][0].node.start_row;
            for capture in &mut header_captures[i] {
                capture.node.end_row = next_header_start_row - 1;
            }
        } else {
            // Last header extends to the end of the file
            for capture in &mut header_captures[i] {
                capture.node.end_row = lines.len() - 1;
            }
        }
    }
    
    // Flatten the grouped captures back to a single array
    header_captures.into_iter().flatten().collect()
}

/// Format markdown captures in exact Codex format
pub fn format_markdown_captures(captures: &[MockCapture], min_section_lines: usize) -> Option<String> {
    if captures.is_empty() {
        return None;
    }
    
    let mut formatted_output = String::new();
    
    // Process all captures (not every other)
    for capture in captures.iter() {
        let start_line = capture.node.start_row;
        let end_line = capture.node.end_row;
        
        // Only include sections that span at least min_section_lines lines
        let section_length = end_line - start_line + 1;
        if section_length >= min_section_lines {
            // Extract header level from the name
            let header_level = if let Some(caps) = Regex::new(r"\.h(\d)$").unwrap().captures(&capture.name) {
                caps.get(1).unwrap().as_str().parse::<usize>().unwrap_or(1)
            } else {
                1
            };
            
            let header_prefix = "#".repeat(header_level);
            
            // Format: startLine--endLine | # Header Text
            // CRITICAL: 1-indexed lines!
            formatted_output.push_str(&format!(
                "{}--{} | {} {}\n",
                start_line + 1,  // Convert to 1-indexed
                end_line + 1,    // Convert to 1-indexed
                header_prefix,
                capture.node.text
            ));
        }
    }
    
    if !formatted_output.is_empty() {
        Some(formatted_output)
    } else {
        None
    }
}

/// Parse markdown and return in exact Codex format
pub fn parse_markdown_to_codex_format(content: &str) -> Option<String> {
    let captures = parse_markdown(content);
    // Use MIN_COMPONENT_LINES = 1 for markdown headers
    format_markdown_captures(&captures, 1)
}
