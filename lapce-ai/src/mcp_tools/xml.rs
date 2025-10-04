// XML Parsing and Generation for MCP Tools
use std::collections::HashMap;
use anyhow::{Result, bail};

#[derive(Debug, Clone)]
pub struct ToolUse {
    pub tool_name: String,
    pub params: HashMap<String, String>,
}

pub fn parse_xml(xml: &str) -> Result<ToolUse> {
    parse_tool_use(xml)
}

pub fn generate_xml(tool_use: &ToolUse) -> String {
    let mut xml = format!("<tool_use>\n<tool_name>{}</tool_name>\n", tool_use.tool_name);
    for (key, value) in &tool_use.params {
        xml.push_str(&format!("<{}>{}</{}>\n", key, value, key));
    }
    xml.push_str("</tool_use>");
    xml
}

pub fn parse_tool_use(xml: &str) -> Result<ToolUse> {
    let mut tool_name = String::new();
    let mut params = HashMap::new();
    let mut current_tag = String::new();
    let mut current_value = String::new();
    let mut in_tag = false;
    let mut is_closing = false;
    
    for ch in xml.chars() {
        match ch {
            '<' => {
                // Save the current value if we have a tag and value
                if !current_value.trim().is_empty() && !current_tag.is_empty() && !is_closing {
                    if current_tag == "tool_name" {
                        tool_name = current_value.trim().to_string();
                    } else if current_tag != "tool_use" {
                        params.insert(current_tag.clone(), current_value.trim().to_string());
                    }
                }
                current_value.clear();
                current_tag.clear();
                in_tag = true;
                is_closing = false;
            }
            '>' => {
                in_tag = false;
                current_tag = current_tag.trim().to_string();
                if current_tag.starts_with('/') {
                    is_closing = true;
                }
            }
            '/' if in_tag => {
                is_closing = true;
            }
            _ if in_tag && !is_closing => {
                current_tag.push(ch);
            }
            _ if !in_tag => {
                current_value.push(ch);
            }
            _ => {}
        }
    }
    
    // Handle any remaining value
    if !current_value.trim().is_empty() && !current_tag.is_empty() && !is_closing {
        if current_tag == "tool_name" {
            tool_name = current_value.trim().to_string();
        } else if current_tag != "tool_use" {
            params.insert(current_tag.clone(), current_value.trim().to_string());
        }
    }
    
    if tool_name.is_empty() {
        bail!("Missing tool_name in XML");
    }
    
    Ok(ToolUse { tool_name, params })
}

pub fn generate_tool_response(tool_name: &str, fields: HashMap<String, String>) -> Result<String> {
    let mut xml = format!("<tool_response>\n<tool_name>{}</tool_name>\n", tool_name);
    
    for (key, value) in fields {
        xml.push_str(&format!("<{}>{}</{}>\n", key, escape_xml(&value), key));
    }
    
    xml.push_str("</tool_response>");
    Ok(xml)
}

fn escape_xml(s: &str) -> String {
    s.chars()
        .map(|ch| match ch {
            '<' => "&lt;".to_string(),
            '>' => "&gt;".to_string(),
            '&' => "&amp;".to_string(),
            '"' => "&quot;".to_string(),
            '\'' => "&apos;".to_string(),
            _ => ch.to_string(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_tool_use() {
        let xml = r#"<tool_use>
            <tool_name>readFile</tool_name>
            <path>/test/file.txt</path>
            <encoding>utf-8</encoding>
        </tool_use>"#;
        
        let result = parse_tool_use(xml).unwrap();
        assert_eq!(result.tool_name, "readFile");
        assert_eq!(result.params.get("path").unwrap(), "/test/file.txt");
        assert_eq!(result.params.get("encoding").unwrap(), "utf-8");
    }
    
    #[test]
    fn test_generate_tool_response() {
        let mut fields = HashMap::new();
        fields.insert("content".to_string(), "Hello <world>".to_string());
        fields.insert("status".to_string(), "success".to_string());
        
        let xml = generate_tool_response("readFile", fields).unwrap();
        assert!(xml.contains("<tool_name>readFile</tool_name>"));
        assert!(xml.contains("Hello &lt;world&gt;"));
    }
}
