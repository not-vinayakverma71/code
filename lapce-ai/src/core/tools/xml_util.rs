use serde_json::{json, Value};
use anyhow::{Result, bail};
use quick_xml::events::{Event, BytesStart, BytesText};
use quick_xml::{Reader, Writer};
use std::io::Cursor;

/// XML parser for tool arguments
pub struct XmlParser;

impl XmlParser {
    pub fn new() -> Self {
        Self
    }
    
    pub fn parse(&self, xml: &str) -> Result<Value> {
        if xml.is_empty() {
            bail!("Empty XML input");
        }
        
        let mut reader = Reader::from_str(xml);
        reader.config_mut().trim_text(true);
        
        let mut stack = vec![];
        let mut current = json!({});
        let mut text_buffer = String::new();
        let mut current_tag = String::new();
        
        loop {
            match reader.read_event() {
                Ok(Event::Start(e)) => {
                    let tag_name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                    
                    if !text_buffer.is_empty() {
                        if !current_tag.is_empty() {
                            current[&current_tag] = json!(text_buffer.trim());
                        }
                        text_buffer.clear();
                    }
                    
                    current_tag = tag_name.clone();
                    stack.push((current_tag.clone(), current.clone()));
                    current = json!({});
                }
                Ok(Event::End(e)) => {
                    let tag_name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                    
                    if !text_buffer.is_empty() {
                        current = json!(text_buffer.trim());
                        text_buffer.clear();
                    }
                    
                    if let Some((parent_tag, mut parent)) = stack.pop() {
                        // Handle arrays (multiple elements with same tag name)
                        if parent.get(&tag_name).is_some() {
                            // Convert to array if not already
                            if !parent[&tag_name].is_array() {
                                let existing = parent[&tag_name].clone();
                                parent[&tag_name] = json!([existing]);
                            }
                            if let Some(arr) = parent[&tag_name].as_array_mut() {
                                arr.push(current);
                            }
                        } else {
                            parent[tag_name] = current;
                        }
                        current = parent;
                        current_tag = parent_tag;
                    }
                }
                Ok(Event::Text(e)) => {
                    text_buffer.push_str(&e.unescape()?.to_string());
                }
                Ok(Event::Eof) => break,
                Err(e) => bail!("XML parsing error: {}", e),
                _ => {}
            }
        }
        
        // Validate that all tags were closed
        if !stack.is_empty() {
            bail!("Unclosed XML tags detected");
        }
        
        Ok(current)
    }
}

/// XML generator for tool responses  
pub struct XmlGenerator;

impl XmlGenerator {
    pub fn new() -> Self {
        Self
    }
    
    pub fn generate(&self, value: &Value) -> Result<String> {
        let mut writer = Writer::new(Cursor::new(Vec::new()));
        
        self.write_value(&mut writer, "root", value)?;
        
        let result = writer.into_inner().into_inner();
        Ok(String::from_utf8(result)?)
    }
    
    fn write_value<W: std::io::Write>(&self, writer: &mut Writer<W>, tag: &str, value: &Value) -> Result<()> {
        match value {
            Value::Object(map) => {
                writer.write_event(Event::Start(BytesStart::new(tag)))?;
                for (key, val) in map {
                    self.write_value(writer, key, val)?;
                }
                writer.write_event(Event::End(quick_xml::events::BytesEnd::new(tag)))?;
            }
            Value::Array(arr) => {
                for item in arr {
                    self.write_value(writer, tag, item)?;
                }
            }
            Value::String(s) => {
                writer.write_event(Event::Start(BytesStart::new(tag)))?;
                writer.write_event(Event::Text(BytesText::new(s)))?;
                writer.write_event(Event::End(quick_xml::events::BytesEnd::new(tag)))?;
            }
            Value::Number(n) => {
                writer.write_event(Event::Start(BytesStart::new(tag)))?;
                writer.write_event(Event::Text(BytesText::new(&n.to_string())))?;
                writer.write_event(Event::End(quick_xml::events::BytesEnd::new(tag)))?;
            }
            Value::Bool(b) => {
                writer.write_event(Event::Start(BytesStart::new(tag)))?;
                writer.write_event(Event::Text(BytesText::new(&b.to_string())))?;
                writer.write_event(Event::End(quick_xml::events::BytesEnd::new(tag)))?;
            }
            Value::Null => {
                writer.write_event(Event::Start(BytesStart::new(tag)))?;
                writer.write_event(Event::End(quick_xml::events::BytesEnd::new(tag)))?;
            }
        }
        
        Ok(())
    }
}

impl Default for XmlParser {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for XmlGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_xml_parser_simple() {
        let parser = XmlParser::new();
        let xml = "<root><name>test</name><value>123</value></root>";
        let result = parser.parse(xml).unwrap();
        
        assert_eq!(result["root"]["name"].as_str().unwrap(), "test");
        assert_eq!(result["root"]["value"].as_str().unwrap(), "123");
    }
    
    #[test]
    fn test_xml_generator_simple() {
        let generator = XmlGenerator::new();
        let data = json!({
            "name": "test",
            "value": 123
        });
        
        let xml = generator.generate(&data).unwrap();
        assert!(xml.contains("<name>test</name>"));
        assert!(xml.contains("<value>123</value>"));
    }
}
