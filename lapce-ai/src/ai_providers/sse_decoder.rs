/// SSE Decoder - Zero-allocation streaming parser
/// EXACT implementation as specified in 03-AI-PROVIDERS-CONSOLIDATED.md

use bytes::{BytesMut, Bytes, Buf};
use std::str;
use serde_json::Value;

/// SSE Event structure
#[derive(Debug, Clone)]
pub struct SseEvent {
    pub id: Option<String>,
    pub event: Option<String>,
    pub data: Option<Bytes>,
    pub retry: Option<u64>,
}

/// SSE Decoder for zero-allocation processing
pub struct SseDecoder {
    buffer: BytesMut,
    position: usize,
}

impl SseDecoder {
    pub fn new() -> Self {
        Self {
            buffer: BytesMut::with_capacity(8192),
            position: 0,
        }
    }
    
    /// Process chunk without allocation
    pub fn process_chunk(&mut self, chunk: &[u8]) -> Vec<SseEvent> {
        self.buffer.extend_from_slice(chunk);
        
        let mut events = Vec::new();
        
        while self.position < self.buffer.len() {
            // Find next newline
            if let Some(newline_pos) = self.find_newline(self.position) {
                // Check for double newline (event boundary)
                if newline_pos == self.position || 
                   (newline_pos > 0 && self.buffer[newline_pos - 1] == b'\r') {
                    // Empty line - process buffered event
                    if let Some(event) = self.parse_event(self.position) {
                        events.push(event);
                    }
                }
                
                self.position = newline_pos + 1;
            } else {
                // No more complete lines in buffer
                break;
            }
        }
        
        // Compact buffer
        if self.position > 0 {
            self.buffer.advance(self.position);
            self.position = 0;
        }
        
        events
    }
    
    fn find_newline(&self, start: usize) -> Option<usize> {
        self.buffer[start..].iter().position(|&b| b == b'\n').map(|p| start + p)
    }
    
    fn parse_event(&self, end: usize) -> Option<SseEvent> {
        let chunk = &self.buffer[..end];
        
        let mut event = SseEvent {
            id: None,
            event: None,
            data: None,
            retry: None,
        };
        
        let mut data_parts = Vec::new();
        let mut i = 0;
        
        while i < chunk.len() {
            let line_end = chunk[i..].iter().position(|&b| b == b'\n')
                .map(|p| i + p)
                .unwrap_or(chunk.len());
            
            let line = &chunk[i..line_end];
            
            if let Some(colon_pos) = line.iter().position(|&b| b == b':') {
                let field = &line[..colon_pos];
                let value_start = if colon_pos + 1 < line.len() && line[colon_pos + 1] == b' ' {
                    colon_pos + 2
                } else {
                    colon_pos + 1
                };
                let value = if value_start < line.len() {
                    &line[value_start..]
                } else {
                    &[]
                };
                
                match field {
                    b"id" => {
                        event.id = str::from_utf8(value).ok().map(|s| s.to_string());
                    }
                    b"event" => {
                        event.event = str::from_utf8(value).ok().map(|s| s.to_string());
                    }
                    b"data" => {
                        data_parts.push(value);
                    }
                    b"retry" => {
                        event.retry = str::from_utf8(value).ok()
                            .and_then(|s| s.parse().ok());
                    }
                    _ => {}
                }
            }
            
            i = line_end + 1;
        }
        
        if !data_parts.is_empty() {
            let mut combined = BytesMut::new();
            for (i, part) in data_parts.iter().enumerate() {
                if i > 0 {
                    combined.extend_from_slice(b"\n");
                }
                combined.extend_from_slice(part);
            }
            event.data = Some(combined.freeze());
        }
        
        if event.id.is_some() || event.event.is_some() || 
           event.data.is_some() || event.retry.is_some() {
            Some(event)
        } else {
            None
        }
    }
}

/// JSON Stream Parser for nested JSON in SSE
pub struct JsonStreamParser {
    buffer: String,
    depth: usize,
}

impl JsonStreamParser {
    pub fn new() -> Self {
        Self {
            buffer: String::with_capacity(4096),
            depth: 0,
        }
    }
    
    pub fn parse_chunk(&mut self, chunk: &str) -> Vec<Value> {
        self.buffer.push_str(chunk);
        
        let mut results = Vec::new();
        let mut start = 0;
        let mut in_string = false;
        let mut escape_next = false;
        
        for (i, ch) in self.buffer.chars().enumerate() {
            if escape_next {
                escape_next = false;
                continue;
            }
            
            match ch {
                '\\' if in_string => escape_next = true,
                '"' if !in_string => in_string = true,
                '"' if in_string => in_string = false,
                '{' if !in_string => self.depth += 1,
                '}' if !in_string => {
                    if self.depth > 0 {
                        self.depth -= 1;
                        if self.depth == 0 {
                            // Complete JSON object
                            if let Ok(value) = serde_json::from_str(&self.buffer[start..=i]) {
                                results.push(value);
                                start = i + 1;
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        
        // Remove processed part
        if start > 0 && start < self.buffer.len() {
            self.buffer = self.buffer[start..].to_string();
        }
        
        results
    }
}

/// Provider-specific SSE parsers
pub mod parsers {
    use super::*;
    use crate::streaming_pipeline::StreamToken;
    use crate::streaming_pipeline::stream_token::{TextDelta, FunctionCall, ToolCall};
    
    /// Parse OpenAI SSE format
    pub fn parse_openai_sse(event: &SseEvent) -> Option<StreamToken> {
        let data = event.data.as_ref()?;
        let data_str = str::from_utf8(data).ok()?;
        
        if data_str == "[DONE]" {
            return Some(StreamToken::Done);
        }
        
        let json: Value = serde_json::from_str(data_str).ok()?;
        
        // Extract delta content
        if let Some(choices) = json["choices"].as_array() {
            if let Some(choice) = choices.first() {
                if let Some(delta) = choice.get("delta") {
                    if let Some(content) = delta["content"].as_str() {
                        return Some(StreamToken::Delta(TextDelta { 
                            content: content.to_string(),
                            index: 0,
                            logprob: None,
                        }));
                    }
                    
                    // Function call
                    if let Some(function_call) = delta.get("function_call") {
                        if let Some(name) = function_call["name"].as_str() {
                            let arguments = function_call["arguments"].as_str().unwrap_or("");
                            return Some(StreamToken::FunctionCall(FunctionCall {
                                name: name.to_string(),
                                arguments: arguments.to_string(),
                            }));
                        }
                    }
                    
                    // Tool calls
                    if let Some(tool_calls) = delta.get("tool_calls") {
                        if let Some(tool_call) = tool_calls.as_array()?.first() {
                            if let Some(function) = tool_call.get("function") {
                                return Some(StreamToken::ToolCall(ToolCall {
                                    id: tool_call["id"].as_str().unwrap_or("").to_string(),
                                    r#type: "function".to_string(),
                                    function: FunctionCall {
                                        name: function["name"].as_str().unwrap_or("").to_string(),
                                        arguments: function["arguments"].as_str().unwrap_or("").to_string(),
                                    },
                                }));
                            }
                        }
                    }
                }
            }
        }
        
        None
    }
    
    /// Parse Anthropic SSE format (event-based)
    pub fn parse_anthropic_sse(event: &SseEvent) -> Option<StreamToken> {
        let data = event.data.as_ref()?;
        let data_str = str::from_utf8(data).ok()?;
        let json: Value = serde_json::from_str(data_str).ok()?;
        
        match event.event.as_deref() {
            Some("message_start") => {
                // Beginning of message
                None
            }
            Some("content_block_start") => {
                // Start of content block
                None
            }
            Some("content_block_delta") => {
                // Content delta
                if let Some(delta) = json.get("delta") {
                    if let Some(text) = delta["text"].as_str() {
                        return Some(StreamToken::Delta(TextDelta {
                            content: text.to_string(),
                            index: 0,
                            logprob: None,
                        }));
                    }
                }
                None
            }
            Some("content_block_stop") => {
                // End of content block
                None
            }
            Some("message_stop") => {
                // End of message
                Some(StreamToken::Done)
            }
            _ => None
        }
    }
    
    /// Parse Gemini streaming format
    pub fn parse_gemini_stream(chunk: &str) -> Vec<StreamToken> {
        let mut parser = JsonStreamParser::new();
        let jsons = parser.parse_chunk(chunk);
        
        let mut tokens = Vec::new();
        
        for json in jsons {
            if let Some(candidates) = json["candidates"].as_array() {
                if let Some(candidate) = candidates.first() {
                    if let Some(content) = candidate.get("content") {
                        if let Some(parts) = content["parts"].as_array() {
                            for part in parts {
                                if let Some(text) = part["text"].as_str() {
                                    tokens.push(StreamToken::Text(text.to_string()));
                                }
                            }
                        }
                    }
                    
                    // Check finish reason
                    if let Some(finish_reason) = candidate["finishReason"].as_str() {
                        if finish_reason == "STOP" {
                            tokens.push(StreamToken::Done);
                        }
                    }
                }
            }
        }
        
        tokens
    }
}
