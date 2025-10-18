/// AI Chat Panel Demo
///
/// Run with: cargo run --example ai_chat_demo
///
/// This demonstrates the AI chat panel with mock LLM responses.
/// Try asking:
/// - "hello"
/// - "how are you"
/// - "what can you do"
/// - "How do I build Lapce?"
/// - "Tell me about Floem"

use floem::{Application, views::Decorators, window::WindowConfig};
use lapce_app::ai_panel_example::ai_chat_panel_view;

fn main() {
    Application::new()
        .window(
            move |_| {
                ai_chat_panel_view()
                    .style(|s| s.width(500.0).height(700.0))
            },
            Some(WindowConfig::default().title("Lapce AI Chat - Demo")),
        )
        .run();
}
