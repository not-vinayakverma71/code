// Message Bubble - Windsurf simple style
// User: blue right-aligned, AI: dark left-aligned

use floem::{
    peniko::Color,
    views::{container, label, Decorators},
    View,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageRole {
    User,
    Assistant,
}

/// User message - Blue bubble, right-aligned
pub fn user_message(text: String) -> impl View {
    container(
        label(move || text.clone())
            .style(|s| {
                s.color(Color::WHITE)
                    .font_size(13.0)
                    .line_height(1.5)
            })
    )
    .style(|s| {
        s.padding(12.0)
            .border_radius(8.0)
            .background(Color::from_rgb8(0x00, 0x78, 0xd4))  // #0078d4 blue
            .max_width_pct(70.0)
    })
}

/// AI message - Dark bubble, left-aligned
pub fn assistant_message(text: String) -> impl View {
    container(
        label(move || text.clone())
            .style(|s| {
                s.color(Color::from_rgb8(0xcc, 0xcc, 0xcc).multiply_alpha(0.9))
                    .font_size(13.0)
                    .line_height(1.5)
            })
    )
    .style(|s| {
        s.padding(12.0)
            .border_radius(8.0)
            .background(Color::from_rgb8(0x1f, 0x1f, 0x1f))  // #1f1f1f dark
            .border(1.0)
            .border_color(Color::from_rgb8(0xff, 0xff, 0xff).multiply_alpha(0.1))
            .max_width_pct(90.0)
    })
}

