/// SSE Parser - Zero-allocation Server-Sent Events parser
/// Phase 1, Task 3: Implement SSE Parser (HARDEST PART!)
/// Based on docs/08-STREAMING-PIPELINE.md lines 100-236

use bytes::{Bytes, BytesMut};
use crate::streaming_pipeline::sse_event::SseEvent;

/// SSE Parser state machine
#[derive(Debug, Clone, PartialEq)]
enum ParseState {
    WaitingForField,
    ParsingField,
    ParsingValue,
    MessageComplete,
}

/// Zero-allocation SSE parser
pub struct SseParser {
    /// Reusable buffer for incoming data
    buffer: BytesMut,
    
    /// Current parsing state
    state: ParseState,
    
    /// Event type buffer
    event_type: String,
    
    /// Data accumulator
    data_buffer: BytesMut,
    
    /// ID buffer
    id_buffer: String,
    
    /// Retry delay
    retry: Option<u64>,
}

impl SseParser {
    /// Create new SSE parser with default buffer sizes
    pub fn new() -> Self {
        Self {
            buffer: BytesMut::with_capacity(8192),
            state: ParseState::WaitingForField,
            event_type: String::with_capacity(32),
            data_buffer: BytesMut::with_capacity(4096),
            id_buffer: String::with_capacity(64),
            retry: None,
        }
    }
    
    /// Parse a chunk of data, returning any complete events
    pub fn parse_chunk(&mut self, chunk: &[u8]) -> Vec<SseEvent> {
        self.buffer.extend_from_slice(chunk);
        let mut events = Vec::new();
        
        // Process all complete lines in buffer
        loop {
            match self.parse_next_event() {
                Some(event) => events.push(event),
                None => break,
            }
        }
        
        events
    }
    
    /// Parse next event from buffer
    fn parse_next_event(&mut self) -> Option<SseEvent> {
        // Find line ending
        let line_end = self.buffer.iter().position(|&b| b == b'\n')?;
        
        // Extract line without allocation
        let line = if line_end > 0 && self.buffer[line_end - 1] == b'\r' {
            &self.buffer[..line_end - 1]
        } else {
            &self.buffer[..line_end]
        };
        
        // Handle different line types
        if line.is_empty() {
            // Empty line - dispatch event if we have data
            if !self.data_buffer.is_empty() || !self.event_type.is_empty() {
                let event = self.build_event();
                self.reset_event_state();
                self.buffer.advance(line_end + 1);
                return Some(event);
            }
        } else if line.starts_with(b":") {
            // Comment - ignore
        } else {
            // Parse field
            self.parse_field(line);
        }
        
        // Advance buffer past this line
        self.buffer.advance(line_end + 1);
        None
    }
    
    /// Parse a field line
    fn parse_field(&mut self, line: &[u8]) {
        // Find colon separator
        if let Some(colon_pos) = line.iter().position(|&b| b == b':') {
            let field = &line[..colon_pos];
            
            // Skip optional space after colon
            let value = if colon_pos + 1 < line.len() && line[colon_pos + 1] == b' ' {
                &line[colon_pos + 2..]
            } else if colon_pos + 1 < line.len() {
                &line[colon_pos + 1..]
            } else {
                &[]
            };
            
            // Process field without allocation
            match field {
                b"data" => {
                    // Add newline if this is not the first data line
                    if !self.data_buffer.is_empty() {
                        self.data_buffer.extend_from_slice(b"\n");
                    }
                    self.data_buffer.extend_from_slice(value);
                }
                b"event" => {
                    self.event_type.clear();
                    if let Ok(s) = std::str::from_utf8(value) {
                        self.event_type.push_str(s);
                    }
                }
                b"id" => {
                    self.id_buffer.clear();
                    if let Ok(s) = std::str::from_utf8(value) {
                        self.id_buffer.push_str(s);
                    }
                }
                b"retry" => {
                    if let Ok(s) = std::str::from_utf8(value) {
                        self.retry = s.parse().ok();
                    }
                }
                _ => {} // Ignore unknown fields
            }
        } else {
            // No colon - treat entire line as field name with empty value
            // This handles cases like "data" or "event" without values
            match line {
                b"data" => {
                    // Empty data field
                }
                _ => {} // Ignore
            }
        }
    }
    
    /// Build event from current buffers
    fn build_event(&self) -> SseEvent {
        SseEvent {
            event_type: if self.event_type.is_empty() {
                None
            } else {
                Some(self.event_type.clone())
            },
            data: Bytes::copy_from_slice(&self.data_buffer),
            id: if self.id_buffer.is_empty() {
                None
            } else {
                Some(self.id_buffer.clone())
            },
            retry: self.retry,
        }
    }
    
    /// Reset event state for next event
    fn reset_event_state(&mut self) {
        self.event_type.clear();
        self.data_buffer.clear();
        self.id_buffer.clear();
        self.retry = None;
        self.state = ParseState::WaitingForField;
    }
    
    /// Get any remaining data that couldn't be parsed
    pub fn remaining(&self) -> &[u8] {
        &self.buffer
    }
    
    /// Clear the parser buffers
    pub fn clear(&mut self) {
        self.buffer.clear();
        self.reset_event_state();
    }
}

impl Default for SseParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_openai_format() {
        let mut parser = SseParser::new();
        
        // OpenAI format
        let input = b"data: {\"text\":\"Hello\"}\n\ndata: {\"text\":\"World\"}\n\ndata: [DONE]\n\n";
        let events = parser.parse_chunk(input);
        
        assert_eq!(events.len(), 3);
        assert_eq!(events[0].data, Bytes::from(&b"{\"text\":\"Hello\"}"[..]));
        assert_eq!(events[1].data, Bytes::from(&b"{\"text\":\"World\"}"[..]));
        assert!(events[2].is_done());
    }
    
    #[test]
    fn test_parse_anthropic_format() {
        let mut parser = SseParser::new();
        
        // Anthropic format with event types
        let input = b"event: message_start\ndata: {\"type\":\"message_start\"}\n\n\
                     event: content_block_delta\ndata: {\"delta\":{\"text\":\"Hi\"}}\n\n\
                     event: message_stop\ndata: {\"type\":\"message_stop\"}\n\n";
        
        let events = parser.parse_chunk(input);
        
        assert_eq!(events.len(), 3);
        assert_eq!(events[0].event_type, Some("message_start".to_string()));
        assert_eq!(events[1].event_type, Some("content_block_delta".to_string()));
        assert_eq!(events[2].event_type, Some("message_stop".to_string()));
    }
    
    #[test]
    fn test_parse_incomplete_chunks() {
        let mut parser = SseParser::new();
        
        // First incomplete chunk
        let chunk1 = b"data: {\"text\":\"Hel";
        let events1 = parser.parse_chunk(chunk1);
        assert_eq!(events1.len(), 0); // No complete event yet
        
        // Second chunk completes the event
        let chunk2 = b"lo\"}\n\n";
        let events2 = parser.parse_chunk(chunk2);
        assert_eq!(events2.len(), 1);
        assert_eq!(events2[0].data, Bytes::from(&b"{\"text\":\"Hello\"}"[..]));
    }
    
    #[test]
    fn test_parse_multiline_data() {
        let mut parser = SseParser::new();
        
        // Multi-line data field
        let input = b"data: line1\ndata: line2\ndata: line3\n\n";
        let events = parser.parse_chunk(input);
        
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].data, Bytes::from(&b"line1\nline2\nline3"[..]));
    }
    
    #[test]
    fn test_parse_with_comments() {
        let mut parser = SseParser::new();
        
        // Input with comments
        let input = b": this is a comment\ndata: actual data\n: another comment\n\n";
        let events = parser.parse_chunk(input);
        
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].data, Bytes::from(&b"actual data"[..]));
    }
    
    #[test]
    fn test_parse_with_id_and_retry() {
        let mut parser = SseParser::new();
        
        let input = b"id: msg-123\nretry: 5000\ndata: test\n\n";
        let events = parser.parse_chunk(input);
        
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].id, Some("msg-123".to_string()));
        assert_eq!(events[0].retry, Some(5000));
    }
}
