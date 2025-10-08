// XML utilities for tool arguments and responses - P0-0: Scaffold core layer

use serde::{Serialize, Deserialize};
use serde_json::Value;
use anyhow::{Result, Context};
use quick_xml::events::{Event, BytesStart, BytesText, BytesEnd};
use quick_xml::{Reader, Writer};
use std::io::Cursor;

/// Represents tool arguments parsed from XML
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct XmlToolArgs {
    pub tool_name: String,
    pub arguments: Value,
    pub multi_file: Option<Vec<FileSpec>>,
}

/// File specification for multi-file operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileSpec {
    pub path: String,
    pub start_line: Option<usize>,
    pub end_line: Option<usize>,
}

/// Parse XML tool arguments
/// Supports both simple and multi-file formats with line ranges
pub fn parse_tool_xml(xml: &str) -> Result<XmlToolArgs> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);
    
    let mut tool_name = String::new();
    let mut arguments = serde_json::Map::new();
    let mut multi_file = Vec::new();
    let mut current_tag = String::new();
    let mut current_file: Option<FileSpec> = None;
    let mut in_file_block = false;
    
    let mut buf = Vec::new();
    
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                
                if name == "tool_use" {
                    // Parse tool name from attribute
                    for attr in e.attributes() {
                        let attr = attr?;
                        if attr.key.as_ref() == b"name" {
                            tool_name = String::from_utf8_lossy(&attr.value).to_string();
                        }
                    }
                } else if name == "file" {
                    in_file_block = true;
                    let mut file_spec = FileSpec {
                        path: String::new(),
                        start_line: None,
                        end_line: None,
                    };
                    
                    // Parse file attributes
                    for attr in e.attributes() {
                        let attr = attr?;
                        match attr.key.as_ref() {
                            b"path" => {
                                file_spec.path = String::from_utf8_lossy(&attr.value).to_string();
                            }
                            b"start_line" => {
                                file_spec.start_line = String::from_utf8_lossy(&attr.value)
                                    .parse()
                                    .ok();
                            }
                            b"end_line" => {
                                file_spec.end_line = String::from_utf8_lossy(&attr.value)
                                    .parse()
                                    .ok();
                            }
                            _ => {}
                        }
                    }
                    
                    current_file = Some(file_spec);
                } else {
                    current_tag = name;
                }
            }
            Ok(Event::Text(e)) => {
                if !current_tag.is_empty() {
                    let text = e.unescape()?.to_string();
                    
                    if in_file_block && current_tag == "path" {
                        if let Some(ref mut file) = current_file {
                            file.path = text;
                        }
                    } else if in_file_block && current_tag == "start_line" {
                        if let Some(ref mut file) = current_file {
                            file.start_line = text.parse().ok();
                        }
                    } else if in_file_block && current_tag == "end_line" {
                        if let Some(ref mut file) = current_file {
                            file.end_line = text.parse().ok();
                        }
                    } else if !in_file_block {
                        // Regular argument
                        arguments.insert(current_tag.clone(), Value::String(text));
                    }
                }
            }
            Ok(Event::End(ref e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                
                if name == "file" {
                    if let Some(file) = current_file.take() {
                        if !file.path.is_empty() {
                            multi_file.push(file);
                        }
                    }
                    in_file_block = false;
                }
                
                current_tag.clear();
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(e).context("Failed to parse XML"),
            _ => {}
        }
        
        buf.clear();
    }
    
    Ok(XmlToolArgs {
        tool_name,
        arguments: Value::Object(arguments),
        multi_file: if multi_file.is_empty() { None } else { Some(multi_file) },
    })
}

/// Generate XML from tool response
pub fn generate_tool_xml(tool_name: &str, result: &Value) -> Result<String> {
    let mut writer = Writer::new(Cursor::new(Vec::new()));
    
    // Start tool_response element
    let mut elem = BytesStart::new("tool_response");
    elem.push_attribute(("name", tool_name));
    writer.write_event(Event::Start(elem))?;
    
    // Write result based on type
    write_value_as_xml(&mut writer, "result", result)?;
    
    // End tool_response
    writer.write_event(Event::End(BytesEnd::new("tool_response")))?;
    
    let result = writer.into_inner().into_inner();
    Ok(String::from_utf8(result)?)
}

fn write_value_as_xml<W: std::io::Write>(
    writer: &mut Writer<W>,
    tag: &str,
    value: &Value,
) -> Result<()> {
    match value {
        Value::String(s) => {
            writer.write_event(Event::Start(BytesStart::new(tag)))?;
            writer.write_event(Event::Text(BytesText::new(s)))?;
            writer.write_event(Event::End(BytesEnd::new(tag)))?;
        }
        Value::Number(n) => {
            writer.write_event(Event::Start(BytesStart::new(tag)))?;
            writer.write_event(Event::Text(BytesText::new(&n.to_string())))?;
            writer.write_event(Event::End(BytesEnd::new(tag)))?;
        }
        Value::Bool(b) => {
            writer.write_event(Event::Start(BytesStart::new(tag)))?;
            writer.write_event(Event::Text(BytesText::new(&b.to_string())))?;
            writer.write_event(Event::End(BytesEnd::new(tag)))?;
        }
        Value::Object(map) => {
            writer.write_event(Event::Start(BytesStart::new(tag)))?;
            for (key, val) in map {
                write_value_as_xml(writer, key, val)?;
            }
            writer.write_event(Event::End(BytesEnd::new(tag)))?;
        }
        Value::Array(arr) => {
            for item in arr {
                write_value_as_xml(writer, tag, item)?;
            }
        }
        Value::Null => {
            let mut elem = BytesStart::new(tag);
            elem.push_attribute(("null", "true"));
            writer.write_event(Event::Empty(elem))?;
        }
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_simple_xml() {
        let xml = r#"
            <tool_use name="readFile">
                <path>/home/user/file.txt</path>
                <encoding>utf-8</encoding>
            </tool_use>
        "#;
        
        let args = parse_tool_xml(xml).unwrap();
        assert_eq!(args.tool_name, "readFile");
        assert_eq!(
            args.arguments.get("path").unwrap(),
            "/home/user/file.txt"
        );
        assert_eq!(
            args.arguments.get("encoding").unwrap(),
            "utf-8"
        );
        assert!(args.multi_file.is_none());
    }
    
    #[test]
    fn test_parse_multi_file_xml() {
        let xml = r#"
            <tool_use name="readFile">
                <file path="file1.txt" start_line="10" end_line="20" />
                <file>
                    <path>file2.txt</path>
                    <start_line>5</start_line>
                    <end_line>15</end_line>
                </file>
                <file>
                    <path>file3.txt</path>
                </file>
            </tool_use>
        "#;
        
        let args = parse_tool_xml(xml).unwrap();
        assert_eq!(args.tool_name, "readFile");
        
        let files = args.multi_file.unwrap();
        assert_eq!(files.len(), 3);
        
        assert_eq!(files[0].path, "file1.txt");
        assert_eq!(files[0].start_line, Some(10));
        assert_eq!(files[0].end_line, Some(20));
        
        assert_eq!(files[1].path, "file2.txt");
        assert_eq!(files[1].start_line, Some(5));
        assert_eq!(files[1].end_line, Some(15));
        
        assert_eq!(files[2].path, "file3.txt");
        assert_eq!(files[2].start_line, None);
        assert_eq!(files[2].end_line, None);
    }
    
    #[test]
    fn test_generate_xml() {
        let result = serde_json::json!({
            "content": "Hello, world!",
            "lines": 42,
            "success": true
        });
        
        let xml = generate_tool_xml("testTool", &result).unwrap();
        
        // Parse it back to verify
        assert!(xml.contains(r#"name="testTool""#));
        assert!(xml.contains("<content>Hello, world!</content>"));
        assert!(xml.contains("<lines>42</lines>"));
        assert!(xml.contains("<success>true</success>"));
    }
    
    #[test]
    fn test_xml_roundtrip() {
        let original = serde_json::json!({
            "message": "Test message",
            "count": 123,
            "active": false,
            "nested": {
                "key1": "value1",
                "key2": 456
            }
        });
        
        let xml = generate_tool_xml("roundtrip", &original).unwrap();
        
        // For this test, we're verifying generation works
        // Full roundtrip would need corresponding parse logic
        assert!(xml.contains("roundtrip"));
        assert!(xml.contains("Test message"));
        assert!(xml.contains("123"));
        assert!(xml.contains("false"));
        assert!(xml.contains("value1"));
        assert!(xml.contains("456"));
    }
}
