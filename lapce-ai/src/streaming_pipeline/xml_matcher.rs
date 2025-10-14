/// XML Matcher for streaming response parsing
use std::collections::VecDeque;

pub struct XmlMatcher {
    buffer: String,
    pending_tags: VecDeque<String>,
}

impl XmlMatcher {
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
            pending_tags: VecDeque::new(),
        }
    }
    
    pub fn push(&mut self, text: String) -> Vec<String> {
        self.buffer.push_str(&text);
        let mut results = Vec::new();
        
        // Look for complete XML elements
        if let Some(start) = self.buffer.find('<') {
            if let Some(tag_end) = self.buffer[start..].find('>') {
                let tag_name_end = start + tag_end + 1;
                
                // Extract tag name
                let tag_content = &self.buffer[start+1..start+tag_end];
                if let Some(space_pos) = tag_content.find(' ') {
                    let tag_name = &tag_content[..space_pos];
                    // Look for closing tag
                    let closing = format!("</{}>", tag_name);
                    if let Some(close_pos) = self.buffer.find(&closing) {
                        let full_element = self.buffer[start..close_pos + closing.len()].to_string();
                        results.push(full_element);
                        self.buffer = self.buffer[close_pos + closing.len()..].to_string();
                    }
                } else if !tag_content.starts_with('/') {
                    // Simple tag without attributes
                    let tag_name = tag_content;
                    let closing = format!("</{}>", tag_name);
                    if let Some(close_pos) = self.buffer.find(&closing) {
                        let full_element = self.buffer[start..close_pos + closing.len()].to_string();
                        results.push(full_element);
                        self.buffer = self.buffer[close_pos + closing.len()..].to_string();
                    }
                }
            }
        }
        
        results
    }
    
    pub fn final_chunks(&mut self) -> Vec<crate::streaming_pipeline::stream_transform::ApiStreamChunk> {
        let mut results = Vec::new();
        
        if !self.buffer.is_empty() {
            results.push(crate::streaming_pipeline::stream_transform::ApiStreamChunk::text(self.buffer.clone()));
            self.buffer.clear();
        }
        
        results
    }
}
