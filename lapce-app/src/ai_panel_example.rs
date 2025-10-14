/// AI Panel Example - Complete chat interface matching Windsurf
///
/// This demonstrates the full AI chat panel with:
/// - Message list with user/AI messages
/// - Inline code and code block rendering
/// - Dim text for AI responses
/// - Input box at bottom
/// - Model selector

use floem::{
    peniko::Color,
    reactive::{create_rw_signal, RwSignal, SignalGet, SignalUpdate},
    unit::PxPctAuto,
    View,
    views::{
        container, h_stack, label, v_stack, Decorators,
    },
};

use crate::ai_theme::{AiTheme, font_size, spacing};
use crate::ai_chat_widgets::{chat_panel, ChatMessage, MessageRole};
use crate::ai_input_widget::chat_input_area;
use crate::ai_mock_llm::{MockLlm, get_quick_response};

/// Complete AI chat panel UI with mock LLM
pub fn ai_chat_panel_view() -> impl View {
    let theme = AiTheme::dark();
    
    // Message state - start with welcome message
    let messages = create_rw_signal(vec![
        ChatMessage {
            role: MessageRole::Assistant,
            content: "Hello! I'm a mock AI assistant for testing Lapce's chat panel. Ask me about Lapce, Floem, or Rust!".to_string(),
            is_streaming: false,
        },
    ]);
    
    let input_text = create_rw_signal(String::new());
    let selected_model = create_rw_signal("Mock LLM".to_string());
    
    // Mock LLM instance
    let mock_llm = create_rw_signal(MockLlm::new());

    let theme_for_header = theme.clone();
    let theme_for_panel = theme.clone();
    let theme_for_input = theme.clone();
    let theme_for_bg = theme.clone();
    
    v_stack((
        // Header with model selector
        chat_header(selected_model, theme_for_header),
        
        // Message list (scrollable)
        chat_panel(messages, theme_for_panel)
            .style(|s| s.flex_grow(1.0)),
        
        // Input area at bottom
        chat_input_with_mock(input_text, messages, mock_llm, theme_for_input),
    ))
    .style(move |s| {
        s.flex_col()
            .width(PxPctAuto::Pct(100.0))
            .height(PxPctAuto::Pct(100.0))
            .background(theme_for_bg.background)
    })
}

/// Chat header with model selector
fn chat_header(selected_model: RwSignal<String>, theme: AiTheme) -> impl View {
    let fg = theme.foreground;
    let hover_bg = theme.hover_background;
    let desc_fg = theme.description_foreground;
    let panel_border = theme.panel_border;
    
    container(
        h_stack((
            label(|| "AI Chat".to_string()).style(move |s| {
                s.font_size(font_size::BASE)
                    .color(fg)
            }),
            
            // Spacer
            floem::views::empty().style(|s| s.flex_grow(1.0)),
            
            // Model selector button
            container(
                label(move || selected_model.get())
            )
            .on_click_stop(move |_| {
                println!("Show model selector");
            })
            .style(move |s| {
                s.font_size(font_size::XS)
                    .padding_horiz(spacing::SPACE_2)
                    .padding_vert(spacing::SPACE_1)
                    .background(hover_bg)
                    .color(desc_fg)
                    .border_radius(spacing::ROUNDED)
                    .cursor(floem::style::CursorStyle::Pointer)
            }),
        ))
    )
    .style(move |s| {
        s.padding(spacing::SPACE_3)
            .border_bottom(1.0)
            .border_color(panel_border)
            .width(PxPctAuto::Pct(100.0))
    })
}

/// Chat input area with mock LLM integration - now using ai_input_widget
fn chat_input_with_mock(
    input_text: RwSignal<String>,
    messages: RwSignal<Vec<ChatMessage>>,
    mock_llm: RwSignal<MockLlm>,
    theme: AiTheme,
) -> impl View {
    let panel_border = theme.panel_border;
    
    container(
        chat_input_area(
            input_text,
            move |text| {
                // Add user message
                messages.update(|msgs| {
                    msgs.push(ChatMessage {
                        role: MessageRole::User,
                        content: text.clone(),
                        is_streaming: false,
                    });
                });
                
                // Check for quick responses first
                if let Some(quick_response) = get_quick_response(&text) {
                    messages.update(|msgs| {
                        msgs.push(ChatMessage {
                            role: MessageRole::Assistant,
                            content: quick_response.to_string(),
                            is_streaming: false,
                        });
                    });
                } else {
                    // Use mock LLM with streaming
                    mock_llm.update(|llm| {
                        llm.stream_response(&text, messages);
                    });
                }
            },
            theme.clone(),
        )
    )
    .style(move |s| {
        s.padding(spacing::SPACE_3)
            .border_top(1.0)
            .border_color(panel_border)
            .width(PxPctAuto::Pct(100.0))
    })
}

/// Model selector dropdown
pub fn model_selector_view() -> impl View {
    let theme = AiTheme::dark();
    let models = vec![
        "GPT-4o",
        "GPT-4 Turbo",
        "Claude 3.5 Sonnet",
        "Claude 3 Opus",
        "Gemini 1.5 Pro",
    ];
    
    floem::views::v_stack_from_iter(
        models.into_iter().map(move |model| {
            container(
                label(move || model.to_string())
            )
            .on_click_stop(move |_| {
                println!("Selected: {}", model);
            })
            .style(move |s| {
                s.width(PxPctAuto::Pct(100.0))
                    .padding(spacing::SPACE_2)
                    .font_size(font_size::SM)
                    .color(theme.foreground)
                    .background(Color::TRANSPARENT)
                    .border_radius(spacing::ROUNDED)
                    .cursor(floem::style::CursorStyle::Pointer)
                    .hover(|s| s.background(theme.hover_background))
            })
        })
    )
    .style(move |s| {
        s.flex_col()
            .padding(spacing::SPACE_2)
            .background(theme.panel_bg)
            .border(1.0)
            .border_color(theme.panel_border)
            .border_radius(spacing::ROUNDED_LG)
            .box_shadow_blur(12.0)
            .box_shadow_color(theme.panel_shadow)
    })
}

/// Integration guide for lapce-app
///
/// To add this to Lapce:
/// 1. Add modules to lapce-app/src/lib.rs or main.rs:
///    ```rust
///    mod ai_theme;
///    mod ai_chat_widgets;
///    mod ai_panel_example;
///    ```
///
/// 2. Create AI panel in your window layout:
///    ```rust
///    use crate::ai_panel_example::ai_chat_panel_view;
///    
///    // In your main view:
///    h_stack((
///        editor_area(),
///        ai_chat_panel_view().style(|s| s.width(400.0)),
///    ))
///    ```
///
/// 3. Wire to IPC bridge (after ai_bridge.rs is ready):
///    ```rust
///    // In chat_input send handler:
///    let ai_bridge = window_data.ai_bridge.clone();
///    ai_bridge.send_message(user_text).await;
///    
///    // Listen for streaming responses:
///    ai_bridge.on_stream_chunk(|chunk| {
///        messages.update(|msgs| {
///            if let Some(last) = msgs.last_mut() {
///                if last.is_streaming {
///                    last.content.push_str(&chunk);
///                }
///            }
///        });
///    });
///    ```
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_theme_creation() {
        let theme = AiTheme::dark();
        assert_eq!(theme.font_size, 13.0);
        assert_eq!(theme.editor_font_size, 14.0);
    }
}
