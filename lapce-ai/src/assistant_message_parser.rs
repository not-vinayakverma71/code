// Assistant Message Parser - CHUNK-03: T11
// Accumulates streaming chunks into AssistantMessageContent

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use tracing::{debug, warn};

use crate::task_exact_translation::AssistantMessageContent;

/// Streaming chunk type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StreamChunk {
    /// Text content chunk
    Text(String),
    
    /// Tool use start marker
    ToolUseStart {
        name: String,
        id: Option<String>,
    },
    
    /// Tool use input chunk (JSON fragment)
    ToolUseInput(String),
    
    /// Tool use end marker
    ToolUseEnd,
    
    /// End of stream
    EndOfStream,
}

/// Parser state for streaming assistant messages
#[derive(Debug, Clone)]
enum ParserState {
    /// Accumulating text content
    Text,
    
    /// Inside a tool use block
    InToolUse {
        name: String,
        id: Option<String>,
        input_buffer: String,
    },
}

/// Assistant message parser for streaming responses
pub struct AssistantMessageParser {
    /// Current parser state
    state: ParserState,
    
    /// Accumulated content blocks
    content_blocks: Vec<AssistantMessageContent>,
    
    /// Current text buffer
    text_buffer: String,
    
    /// Whether stream has ended
    stream_ended: bool,
    
    /// Chunk queue for lookahead
    chunk_queue: VecDeque<StreamChunk>,
}

impl AssistantMessageParser {
    /// Create a new parser
    pub fn new() -> Self {
        Self {
            state: ParserState::Text,
            content_blocks: Vec::new(),
            text_buffer: String::new(),
            stream_ended: false,
            chunk_queue: VecDeque::new(),
        }
    }
    
    /// Process a chunk from the stream
    pub fn process_chunk(&mut self, chunk: StreamChunk) -> Result<(), String> {
        if self.stream_ended {
            return Err("Cannot process chunks after stream has ended".to_string());
        }
        
        match chunk {
            StreamChunk::Text(text) => {
                self.handle_text_chunk(text);
            }
            StreamChunk::ToolUseStart { name, id } => {
                self.handle_tool_use_start(name, id)?;
            }
            StreamChunk::ToolUseInput(input) => {
                self.handle_tool_use_input(input)?;
            }
            StreamChunk::ToolUseEnd => {
                self.handle_tool_use_end()?;
            }
            StreamChunk::EndOfStream => {
                self.handle_end_of_stream();
            }
        }
        
        Ok(())
    }
    
    /// Handle text chunk
    fn handle_text_chunk(&mut self, text: String) {
        match &self.state {
            ParserState::Text => {
                self.text_buffer.push_str(&text);
            }
            ParserState::InToolUse { .. } => {
                warn!("Received text chunk while in tool use state, ignoring");
            }
        }
    }
    
    /// Handle tool use start
    fn handle_tool_use_start(&mut self, name: String, id: Option<String>) -> Result<(), String> {
        // Flush any pending text
        self.flush_text_buffer();
        
        // Transition to tool use state
        self.state = ParserState::InToolUse {
            name,
            id,
            input_buffer: String::new(),
        };
        
        debug!("Started tool use block");
        Ok(())
    }
    
    /// Handle tool use input chunk
    fn handle_tool_use_input(&mut self, input: String) -> Result<(), String> {
        match &mut self.state {
            ParserState::InToolUse { input_buffer, .. } => {
                input_buffer.push_str(&input);
                Ok(())
            }
            ParserState::Text => {
                Err("Received tool use input while not in tool use state".to_string())
            }
        }
    }
    
    /// Handle tool use end
    fn handle_tool_use_end(&mut self) -> Result<(), String> {
        match std::mem::replace(&mut self.state, ParserState::Text) {
            ParserState::InToolUse { name, input_buffer, .. } => {
                // Parse input as JSON
                let input = serde_json::from_str(&input_buffer)
                    .unwrap_or_else(|_| {
                        // Fallback to raw string if not valid JSON
                        serde_json::Value::String(input_buffer)
                    });
                
                // Add tool use block
                self.content_blocks.push(AssistantMessageContent::ToolUse {
                    name,
                    input,
                });
                
                debug!("Completed tool use block");
                Ok(())
            }
            ParserState::Text => {
                Err("Received tool use end while not in tool use state".to_string())
            }
        }
    }
    
    /// Handle end of stream
    fn handle_end_of_stream(&mut self) {
        // Flush any remaining text
        self.flush_text_buffer();
        
        // Mark stream as ended
        self.stream_ended = true;
        debug!("Stream ended");
    }
    
    /// Flush text buffer to content blocks
    fn flush_text_buffer(&mut self) {
        if !self.text_buffer.is_empty() {
            self.content_blocks.push(AssistantMessageContent::Text {
                text: self.text_buffer.clone(),
            });
            self.text_buffer.clear();
        }
    }
    
    /// Get all accumulated content blocks
    pub fn get_content(&self) -> Vec<AssistantMessageContent> {
        self.content_blocks.clone()
    }
    
    /// Check if stream has ended
    pub fn is_stream_ended(&self) -> bool {
        self.stream_ended
    }
    
    /// Get content block count
    pub fn block_count(&self) -> usize {
        self.content_blocks.len()
    }
    
    /// Reset parser state
    pub fn reset(&mut self) {
        self.state = ParserState::Text;
        self.content_blocks.clear();
        self.text_buffer.clear();
        self.stream_ended = false;
        self.chunk_queue.clear();
    }
    
    /// Parse from raw text stream (simple text-only mode)
    pub fn parse_text_stream(&mut self, text: String) -> Result<(), String> {
        self.process_chunk(StreamChunk::Text(text))
    }
    
    /// Finalize and get result
    pub fn finalize(mut self) -> Vec<AssistantMessageContent> {
        if !self.stream_ended {
            self.handle_end_of_stream();
        }
        self.content_blocks
    }
}

impl Default for AssistantMessageParser {
    fn default() -> Self {
        Self::new()
    }
}

/// Simple text-based parser (fallback for non-streaming)
pub fn parse_assistant_text(text: &str) -> Vec<AssistantMessageContent> {
    vec![AssistantMessageContent::Text {
        text: text.to_string(),
    }]
}

/// Parse tool invocations from XML-like format
/// Format: <tool_name>{"param": "value"}</tool_name>
pub fn parse_xml_tool_invocation(xml: &str) -> Result<AssistantMessageContent, String> {
    // Simple regex-based parsing for XML tool format
    let pattern = regex::Regex::new(r"<(\w+)>(.*?)</\1>")
        .map_err(|e| format!("Regex error: {}", e))?;
    
    if let Some(captures) = pattern.captures(xml) {
        let tool_name = captures.get(1)
            .ok_or("Missing tool name")?
            .as_str()
            .to_string();
        
        let json_str = captures.get(2)
            .ok_or("Missing tool input")?
            .as_str();
        
        let input = serde_json::from_str(json_str)
            .map_err(|e| format!("Failed to parse tool input: {}", e))?;
        
        Ok(AssistantMessageContent::ToolUse {
            name: tool_name,
            input,
        })
    } else {
        Err("Invalid tool invocation format".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parser_creation() {
        let parser = AssistantMessageParser::new();
        assert_eq!(parser.block_count(), 0);
        assert!(!parser.is_stream_ended());
    }
    
    #[test]
    fn test_text_only_stream() {
        let mut parser = AssistantMessageParser::new();
        
        parser.process_chunk(StreamChunk::Text("Hello ".to_string())).unwrap();
        parser.process_chunk(StreamChunk::Text("world!".to_string())).unwrap();
        parser.process_chunk(StreamChunk::EndOfStream).unwrap();
        
        let content = parser.get_content();
        assert_eq!(content.len(), 1);
        
        match &content[0] {
            AssistantMessageContent::Text { text } => {
                assert_eq!(text, "Hello world!");
            }
            _ => panic!("Expected text content"),
        }
    }
    
    #[test]
    fn test_tool_use_stream() {
        let mut parser = AssistantMessageParser::new();
        
        parser.process_chunk(StreamChunk::ToolUseStart {
            name: "read_file".to_string(),
            id: Some("tool_1".to_string()),
        }).unwrap();
        
        parser.process_chunk(StreamChunk::ToolUseInput(
            r#"{"path": "test.txt"}"#.to_string()
        )).unwrap();
        
        parser.process_chunk(StreamChunk::ToolUseEnd).unwrap();
        parser.process_chunk(StreamChunk::EndOfStream).unwrap();
        
        let content = parser.get_content();
        assert_eq!(content.len(), 1);
        
        match &content[0] {
            AssistantMessageContent::ToolUse { name, input } => {
                assert_eq!(name, "read_file");
                assert_eq!(input["path"], "test.txt");
            }
            _ => panic!("Expected tool use content"),
        }
    }
    
    #[test]
    fn test_mixed_content_stream() {
        let mut parser = AssistantMessageParser::new();
        
        // Text
        parser.process_chunk(StreamChunk::Text("I'll read the file. ".to_string())).unwrap();
        
        // Tool use
        parser.process_chunk(StreamChunk::ToolUseStart {
            name: "read_file".to_string(),
            id: None,
        }).unwrap();
        parser.process_chunk(StreamChunk::ToolUseInput(
            r#"{"path": "test.txt"}"#.to_string()
        )).unwrap();
        parser.process_chunk(StreamChunk::ToolUseEnd).unwrap();
        
        // More text
        parser.process_chunk(StreamChunk::Text("Done!".to_string())).unwrap();
        
        parser.process_chunk(StreamChunk::EndOfStream).unwrap();
        
        let content = parser.get_content();
        assert_eq!(content.len(), 3);
        
        // Verify order
        assert!(matches!(content[0], AssistantMessageContent::Text { .. }));
        assert!(matches!(content[1], AssistantMessageContent::ToolUse { .. }));
        assert!(matches!(content[2], AssistantMessageContent::Text { .. }));
    }
    
    #[test]
    fn test_incremental_tool_input() {
        let mut parser = AssistantMessageParser::new();
        
        parser.process_chunk(StreamChunk::ToolUseStart {
            name: "write_file".to_string(),
            id: None,
        }).unwrap();
        
        // Input arrives in chunks
        parser.process_chunk(StreamChunk::ToolUseInput(r#"{"path":"#.to_string())).unwrap();
        parser.process_chunk(StreamChunk::ToolUseInput(r#"test.txt","#.to_string())).unwrap();
        parser.process_chunk(StreamChunk::ToolUseInput(r#""content":"Hello"}"#.to_string())).unwrap();
        
        parser.process_chunk(StreamChunk::ToolUseEnd).unwrap();
        parser.process_chunk(StreamChunk::EndOfStream).unwrap();
        
        let content = parser.get_content();
        assert_eq!(content.len(), 1);
        
        match &content[0] {
            AssistantMessageContent::ToolUse { name, input } => {
                assert_eq!(name, "write_file");
                assert_eq!(input["path"], "test.txt");
                assert_eq!(input["content"], "Hello");
            }
            _ => panic!("Expected tool use"),
        }
    }
    
    #[test]
    fn test_reset() {
        let mut parser = AssistantMessageParser::new();
        
        parser.process_chunk(StreamChunk::Text("Test".to_string())).unwrap();
        parser.process_chunk(StreamChunk::EndOfStream).unwrap();
        
        assert_eq!(parser.block_count(), 1);
        assert!(parser.is_stream_ended());
        
        parser.reset();
        
        assert_eq!(parser.block_count(), 0);
        assert!(!parser.is_stream_ended());
    }
    
    #[test]
    fn test_cannot_process_after_end() {
        let mut parser = AssistantMessageParser::new();
        
        parser.process_chunk(StreamChunk::EndOfStream).unwrap();
        
        let result = parser.process_chunk(StreamChunk::Text("Late".to_string()));
        assert!(result.is_err());
    }
    
    #[test]
    fn test_parse_simple_text() {
        let content = parse_assistant_text("Hello, world!");
        assert_eq!(content.len(), 1);
        
        match &content[0] {
            AssistantMessageContent::Text { text } => {
                assert_eq!(text, "Hello, world!");
            }
            _ => panic!("Expected text"),
        }
    }
    
    #[test]
    fn test_parse_xml_tool() {
        let xml = r#"<read_file>{"path": "test.txt"}</read_file>"#;
        let content = parse_xml_tool_invocation(xml).unwrap();
        
        match content {
            AssistantMessageContent::ToolUse { name, input } => {
                assert_eq!(name, "read_file");
                assert_eq!(input["path"], "test.txt");
            }
            _ => panic!("Expected tool use"),
        }
    }
    
    #[test]
    fn test_finalize() {
        let mut parser = AssistantMessageParser::new();
        parser.process_chunk(StreamChunk::Text("Test".to_string())).unwrap();
        
        // Finalize without explicit EndOfStream
        let content = parser.finalize();
        assert_eq!(content.len(), 1);
    }
}
