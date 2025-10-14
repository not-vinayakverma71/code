// Simple Chat Panel Demo - Windsurf-style
// Shows: user message, AI message, code block, file link

use floem::{
    peniko::Color,
    reactive::{create_rw_signal, SignalGet, SignalUpdate},
    views::{
        container, h_stack, label, scroll, v_stack, Decorators, text_input,
    },
    Application, View,
};

fn main() {
    Application::new()
        .window(
            |_| {
                chat_demo_view()
            },
            None,
        )
        .run();
}

fn chat_demo_view() -> impl View {

    v_stack((
        // Header
        container(
            label(|| "Chat Demo - Windsurf Style".to_string())
                .style(|s| {
                    s.font_size(16.0)
                        .font_weight(floem::text::Weight::BOLD)
                        .color(Color::WHITE)
                })
        )
        .style(|s| {
            s.width_full()
                .padding(16.0)
                .background(Color::from_rgb8(0x20, 0x20, 0x20))
                .border_bottom(1.0)
                .border_color(Color::from_rgb8(0x45, 0x45, 0x45))
        }),

        // Messages area
        scroll(
            container(
                v_stack((
                    user_message("How do I read a file in Rust?".to_string()),
                    assistant_message("I'll help you with that! Here's how to read a file:".to_string()),
                    code_block("rust".to_string(), r#"use std::fs;

fn read_file(path: &str) -> std::io::Result<String> {
    fs::read_to_string(path)
}

let content = read_file("example.txt")?;
println!("{}", content);"#.to_string()),
                    assistant_message("I've also checked your code. See:".to_string()),
                    file_link("src/main.rs".to_string()),
                    user_message("Thanks! Can you show me error handling?".to_string()),
                    assistant_message("Sure! Here's a better version with error handling:".to_string()),
                    code_block("rust".to_string(), r#"use std::fs::File;
use std::io::{self, Read};

fn read_file_safe(path: &str) -> io::Result<String> {
    let mut file = File::open(path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    Ok(contents)
}"#.to_string()),
                ))
                .style(|s| s.flex_col().gap(12.0))
            )
            .style(|s| s.padding(16.0))
        )
        .style(|s| s.flex_grow(1.0)),

        // Input area
        container(
            h_stack((
                text_input(create_rw_signal(String::new()))
                    .placeholder("Type a message...".to_string())
                    .style(|s| {
                        s.flex_grow(1.0)
                            .padding(12.0)
                            .background(Color::from_rgb8(0x31, 0x31, 0x31))
                            .border(1.0)
                            .border_color(Color::from_rgb8(0x3c, 0x3c, 0x3c))
                            .border_radius(6.0)
                            .color(Color::from_rgb8(0xcc, 0xcc, 0xcc))
                    }),
                
                // Send button
                container(
                    label(|| "↑".to_string())
                        .style(|s| {
                            s.color(Color::WHITE)
                                .font_size(16.0)
                                .font_weight(floem::text::Weight::BOLD)
                        })
                )
                .style(|s| {
                    s.width(32.0)
                        .height(32.0)
                        .border_radius(16.0)
                        .background(Color::from_rgb8(0x00, 0x78, 0xd4))
                        .justify_center()
                        .items_center()
                        .cursor(floem::style::CursorStyle::Pointer)
                        .hover(|s| s.background(Color::from_rgb8(0x02, 0x6e, 0xc1)))
                }),
            ))
            .style(|s| s.gap(8.0))
        )
        .style(|s| {
            s.width_full()
                .padding(16.0)
                .background(Color::from_rgb8(0x20, 0x20, 0x20))
                .border_top(1.0)
                .border_color(Color::from_rgb8(0x45, 0x45, 0x45))
        }),
    ))
    .style(|s| {
        s.width_full()
            .height_full()
            .background(Color::from_rgb8(0x1e, 0x1e, 0x1e))
    })
}


fn user_message(text: String) -> impl View {
    container(
        label(move || text.clone())
            .style(|s| {
                s.color(Color::WHITE)
                    .font_size(13.0)
            })
    )
    .style(|s| {
        s.padding(12.0)
            .border_radius(8.0)
            .background(Color::from_rgb8(0x00, 0x78, 0xd4))
            .max_width_pct(70.0)
    })
}

fn assistant_message(text: String) -> impl View {
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
            .background(Color::from_rgb8(0x1f, 0x1f, 0x1f))
            .border(1.0)
            .border_color(Color::from_rgb8(0xff, 0xff, 0xff).multiply_alpha(0.1))
            .max_width_pct(90.0)
    })
}

fn code_block(language: String, code: String) -> impl View {
    v_stack((
        // Header
        h_stack((
            label(move || language.clone())
                .style(|s| {
                    s.font_size(12.0)
                        .color(Color::from_rgb8(0xcc, 0xcc, 0xcc).multiply_alpha(0.7))
                }),
            
            container(floem::views::empty()).style(|s| s.flex_grow(1.0)),
            
            // Copy button
            container(
                label(|| "📋 Copy".to_string())
            )
            .on_click_stop(move |_| {
                println!("Copied code!");
                // TODO: Copy to clipboard
            })
            .style(|s| {
                s.padding(4.0)
                    .padding_horiz(8.0)
                    .border_radius(4.0)
                    .font_size(11.0)
                    .color(Color::from_rgb8(0xcc, 0xcc, 0xcc).multiply_alpha(0.7))
                    .cursor(floem::style::CursorStyle::Pointer)
                    .hover(|s| {
                        s.background(Color::from_rgb8(0xff, 0xff, 0xff).multiply_alpha(0.1))
                            .color(Color::from_rgb8(0xcc, 0xcc, 0xcc))
                    })
            }),
        ))
        .style(|s| {
            s.width_full()
                .padding(8.0)
                .items_center()
                .background(Color::from_rgb8(0x20, 0x20, 0x20))
                .border_bottom(1.0)
                .border_color(Color::from_rgb8(0x45, 0x45, 0x45))
        }),
        
        // Code content
        scroll(
            container(
                label(move || code.clone())
                    .style(|s| {
                        s.font_family("monospace".to_string())
                            .font_size(13.0)
                            .line_height(1.5)
                            .color(Color::from_rgb8(0xcc, 0xcc, 0xcc))
                    })
            )
            .style(|s| s.padding(12.0))
        )
        .style(|s| {
            s.width_full()
                .max_height(400.0)
                .background(Color::from_rgb8(0x20, 0x20, 0x20))
        }),
    ))
    .style(|s| {
        s.width_full()
            .border_radius(6.0)
            .border(1.0)
            .border_color(Color::from_rgb8(0x45, 0x45, 0x45))
            .max_width_pct(90.0)
    })
}

fn file_link(path: String) -> impl View {
    let filename = path.split('/').last().unwrap_or(&path).to_string();
    
    h_stack((
        label(|| "Read ".to_string())
            .style(|s| {
                s.font_size(13.0)
                    .color(Color::from_rgb8(0xcc, 0xcc, 0xcc))
            }),
        
        container(
            label(move || filename.clone())
                .style(|s| {
                    s.font_size(13.0)
                        .font_family("monospace".to_string())
                        .color(Color::from_rgb8(0x40, 0xa6, 0xff))
                })
        )
        .on_click_stop(move |_| {
            println!("Open file: {}", path);
            // TODO: Open file in editor
        })
        .style(|s| {
            s.cursor(floem::style::CursorStyle::Pointer)
                .hover(|s| {
                    s.border_bottom(1.0)
                        .border_color(Color::from_rgb8(0x40, 0xa6, 0xff))
                })
        }),
    ))
    .style(|s| {
        s.items_baseline()
            .gap(0.0)
    })
}
