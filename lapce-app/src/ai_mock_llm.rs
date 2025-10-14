/// Mock LLM for testing AI chat panel
///
/// Simulates AI responses with realistic delays and streaming

use floem::reactive::{RwSignal, SignalUpdate};
use crate::ai_chat_widgets::{ChatMessage, MessageRole};

/// Mock LLM that generates responses
pub struct MockLlm {
    responses: Vec<&'static str>,
    response_index: usize,
}

impl MockLlm {
    pub fn new() -> Self {
        Self {
            responses: vec![
                "Lapce is a lightning-fast code editor written in Rust. It uses `Floem` for the UI framework and `wgpu` for GPU-accelerated rendering.\n\nHere's a basic example:\n```rust\nuse floem::views::*;\n\nfn main() {\n    v_stack((\n        label(\"Hello Lapce!\"),\n        button(\"Click me\"),\n    ))\n}\n```",
                
                "To build Lapce from source:\n\n1. Install Rust: `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`\n2. Clone the repo: `git clone https://github.com/lapce/lapce.git`\n3. Build: `cargo build --release`\n\nThe binary will be in `target/release/lapce`.",
                
                "Floem provides several layout widgets:\n\n- `v_stack()` - vertical stack\n- `h_stack()` - horizontal stack\n- `scroll()` - scrollable container\n- `container()` - generic container\n\nYou can style them with `.style(|s| s.padding(10.0).background(Color::WHITE))`",
                
                "The AI features in Lapce use an IPC architecture:\n\n```\nLapce UI (Floem)\n    ↓ IPC\nlapce-ai backend (Rust)\n    ↓ HTTP/API\nLLM providers (OpenAI, Anthropic, etc.)\n```\n\nThis keeps the UI responsive while AI processes in the background.",
                
                "To add custom themes in Lapce:\n\n1. Create a `.toml` file in `~/.config/lapce/themes/`\n2. Define colors:\n```toml\n[ui]\nbackground = \"#1e1e1e\"\nforeground = \"#d4d4d4\"\n\n[syntax]\nkeyword = \"#569cd6\"\nstring = \"#ce9178\"\n```\n3. Reload Lapce and select your theme in settings.",
                
                "Lapce supports LSP (Language Server Protocol) for:\n- Auto-completion\n- Go to definition\n- Find references\n- Diagnostics (errors/warnings)\n- Code actions\n\nIt works with any LSP server like `rust-analyzer`, `typescript-language-server`, etc.",
            ],
            response_index: 0,
        }
    }

    /// Generate a response for the given user message
    pub fn generate_response(&mut self, _user_message: &str) -> String {
        let response = self.responses[self.response_index].to_string();
        self.response_index = (self.response_index + 1) % self.responses.len();
        response
    }

    /// Simulate response (non-streaming to keep UI thread-safe)
    pub fn stream_response(
        &mut self,
        user_message: &str,
        messages: RwSignal<Vec<ChatMessage>>,
    ) {
        let response = self.generate_response(user_message);
        
        messages.update(|msgs| {
            msgs.push(ChatMessage {
                role: MessageRole::Assistant,
                content: response,
                is_streaming: false,
            });
        });
    }
}

impl Default for MockLlm {
    fn default() -> Self {
        Self::new()
    }
}

/// Simple responses for common queries
pub fn get_quick_response(query: &str) -> Option<&'static str> {
    let query_lower = query.to_lowercase();
    
    if query_lower.contains("hello") || query_lower.contains("hi") {
        Some("Hello! I'm a mock AI assistant for testing Lapce's chat panel. Ask me about Lapce, Floem, or Rust!")
    } else if query_lower.contains("how are you") {
        Some("I'm doing great! I'm a test AI running inside Lapce. How can I help you today?")
    } else if query_lower.contains("what can you do") {
        Some("I can help you with:\n- Lapce features and setup\n- Floem UI framework\n- Rust programming\n- Code examples and explanations\n\nJust ask me anything!")
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_llm_responses() {
        let mut llm = MockLlm::new();
        let response1 = llm.generate_response("test");
        let response2 = llm.generate_response("test");
        
        assert!(!response1.is_empty());
        assert!(!response2.is_empty());
        assert_ne!(response1, response2); // Should cycle through responses
    }

    #[test]
    fn test_quick_responses() {
        assert!(get_quick_response("hello").is_some());
        assert!(get_quick_response("how are you").is_some());
        assert!(get_quick_response("random query").is_none());
    }
}
