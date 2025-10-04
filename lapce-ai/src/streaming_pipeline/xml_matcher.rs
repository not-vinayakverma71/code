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
        
        // Simple XML tag detection
        while let Some(start) = self.buffer.find('<') {
            if let Some(end) = self.buffer.find('>') {
                if end > start {
                    let tag = self.buffer[start..=end].to_string();
                    results.push(tag.clone());
                    self.buffer = self.buffer[end+1..].to_string();
                } else {
                    break;
                }
            } else {
                break;
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
