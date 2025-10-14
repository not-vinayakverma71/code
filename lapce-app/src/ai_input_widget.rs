/// AI Chat Input Widget - Windsurf-styled input box
///
/// Features:
/// - Multi-line text input with auto-resize
/// - Focus state with color change (#3c3c3c → #0078d4)
/// - Send button with hover states

use floem::{
    reactive::{RwSignal, SignalGet, SignalUpdate},
    unit::PxPctAuto,
    View,
    views::{
        container, text_input, h_stack, Decorators,
    },
    keyboard::{Key, NamedKey},
    event::{EventListener},
};

use crate::ai_theme::{AiTheme, font_size, spacing};

/// Chat input area with send button
///
/// Matches Windsurf input styling:
/// - Background: #313131
/// - Border: #3c3c3c
/// - Focus border: #0078d4 (blue)
/// - Placeholder: #989898
pub fn chat_input_area(
    input_text: RwSignal<String>,
    on_send: impl Fn(String) + 'static + Clone,
    theme: AiTheme,
) -> impl View {
    let theme_input = theme.clone();
    let theme_button = theme.clone();
    let on_send_clone = on_send.clone();
    
    h_stack((
        // Text input
        text_input(input_text)
            .placeholder("Ask anything...")
            .on_event_stop(EventListener::KeyDown, move |event| {
                if let floem::event::Event::KeyDown(key_event) = event {
                    if key_event.key.logical_key == Key::Named(NamedKey::Enter) 
                        && !key_event.modifiers.shift() 
                    {
                        let text = input_text.get();
                        if !text.trim().is_empty() {
                            on_send_clone(text);
                            input_text.set(String::new());
                        }
                    }
                }
            })
            .style(move |s| {
                s.background(theme_input.input_background)       // #313131
                    .color(theme_input.input_foreground)         // #cccccc
                    .border(1.0)
                    .border_color(theme_input.input_border)      // #3c3c3c
                    .border_radius(spacing::ROUNDED_MD)          // 6px
                    .padding_horiz(spacing::SPACE_3)             // 12px
                    .padding_vert(spacing::SPACE_2)              // 8px
                    .font_size(font_size::SM)
                    .font_family(theme_input.font_family.to_string())
                    .min_height(spacing::INPUT_HEIGHT_MIN)       // 36px
                    .width(PxPctAuto::Pct(100.0))
                    .focus(move |s| {
                        s.border_color(theme_input.input_focus_border)  // #0078d4 blue
                            .outline(0.0)
                    })
                    // Note: Placeholder color (#989898) is handled by Floem internally
            }),
        
        // Send button
        send_button(
            move || {
                let text = input_text.get();
                if !text.trim().is_empty() {
                    on_send(text);
                    input_text.set(String::new());
                }
            },
            theme_button,
        ),
    ))
    .style(move |s| {
        s.gap(spacing::SPACE_2)  // 8px gap between input and button
            .width(PxPctAuto::Pct(100.0))
            .align_items(Some(floem::style::AlignItems::Center))
    })
}

/// Send button - Windsurf circular style with up arrow
///
/// Circle: 32x32px, fully rounded
/// Primary: #0078d4
/// Hover: #026ec1
/// Icon: ↑ (up arrow)
pub fn send_button(
    on_click: impl Fn() + 'static,
    theme: AiTheme,
) -> impl View {
    container(
        floem::views::label(|| "↑".to_string())
            .style(move |s| {
                s.color(theme.button_primary_foreground)  // #ffffff
                    .font_size(16.0)  // Slightly larger for icon
                    .font_weight(floem::text::Weight::BOLD)
            })
    )
    .on_click_stop(move |_| {
        on_click();
    })
    .style(move |s| {
        s.background(theme.button_primary)                // #0078d4
            .border_radius(100.0)                         // Fully circular
            .width(32.0)                                  // Fixed circle size
            .height(32.0)
            .justify_center()
            .items_center()
            .cursor(floem::style::CursorStyle::Pointer)
            .hover(move |s| {
                s.background(theme.button_primary_hover)  // #026ec1
            })
            .active(move |s| {
                s.background(theme.button_primary_hover)
                    .apply_if(true, |s| s.scale(0.95))
            })
    })
}

/// Secondary button (e.g., Cancel, Clear)
///
/// Background: #313131
/// Hover: #3c3c3c
pub fn secondary_button(
    label: impl Fn() -> String + 'static,
    on_click: impl Fn() + 'static,
    theme: AiTheme,
) -> impl View {
    let label_theme = theme.clone();
    
    container(
        floem::views::label(label)
            .style(move |s| {
                s.color(label_theme.button_secondary_foreground)  // #cccccc
                    .font_size(font_size::SM)
                    .font_weight(floem::text::Weight::MEDIUM)
            })
    )
    .on_click_stop(move |_| {
        on_click();
    })
    .style(move |s| {
        s.background(theme.button_secondary)              // #313131
            .border_radius(spacing::ROUNDED_MD)           // 6px
            .padding_horiz(spacing::SPACE_4)              // 16px
            .padding_vert(spacing::SPACE_2)               // 8px
            .min_height(spacing::BUTTON_HEIGHT)           // 28px
            .cursor(floem::style::CursorStyle::Pointer)
            .hover(move |s| {
                s.background(theme.button_secondary_hover)  // #3c3c3c
            })
            .active(move |s| {
                s.background(theme.button_secondary_hover)
                    .apply_if(true, |s| s.scale(0.98))
            })
    })
}

/// Icon button (small, for actions like copy, regenerate)
pub fn icon_button(
    icon: impl Fn() -> String + 'static,
    tooltip: &'static str,
    on_click: impl Fn() + 'static,
    theme: AiTheme,
) -> impl View {
    let icon_theme = theme.clone();
    
    container(
        floem::views::label(icon)
            .style(move |s| {
                s.color(icon_theme.foreground.multiply_alpha(0.7))
                    .font_size(font_size::BASE)
            })
    )
    .on_click_stop(move |_| {
        on_click();
    })
    .style(move |s| {
        s.padding(spacing::SPACE_2)
            .border_radius(spacing::ROUNDED)
            .cursor(floem::style::CursorStyle::Pointer)
            .hover(move |s| {
                s.background(theme.hover_background)
            })
            .active(move |s| {
                s.background(theme.active_background)
            })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_input_creates_without_panic() {
        // Basic smoke test
        let input_text = RwSignal::new(String::new());
        let _view = chat_input_area(input_text, |_| {}, AiTheme::dark());
    }

    #[test]
    fn test_buttons_create_without_panic() {
        let theme = AiTheme::dark();
        let _primary = send_button(|| {}, theme.clone());
        let _secondary = secondary_button(|| "Cancel".to_string(), || {}, theme.clone());
        let _icon = icon_button(|| "⚙".to_string(), "Settings", || {}, theme);
    }
}
