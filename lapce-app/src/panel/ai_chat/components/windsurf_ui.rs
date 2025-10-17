// Clean Windsurf-style UI components
// Extracted from windsurf_chat_demo.rs

use floem::{
    peniko::Color,
    reactive::{create_rw_signal, RwSignal, SignalGet, SignalUpdate},
    views::{
        container, h_stack, label, v_stack, svg, Decorators,
    },
    View,
};

// EXACT Windsurf user message from outerHTML
// Right-aligned: flex justify-end, rounded-[8px] border bg-ide-input-background px-[9px] py-[6px]
pub fn user_message(text: String) -> impl View {
    container(
        container(
            label(move || text.clone())
                .style(|s| {
                    s.font_size(12.0)
                        .color(Color::from_rgb8(0xcc, 0xcc, 0xcc))
                        .line_height(1.5)
                        .text_overflow(floem::style::TextOverflow::Wrap)
                })
        )
        .style(|s| {
            s.border_radius(8.0)
                .border(1.0)
                .border_color(Color::from_rgb8(0x7d, 0x7d, 0x7d).multiply_alpha(0.125))
                .background(Color::from_rgb8(0x31, 0x31, 0x31))
                .padding_horiz(9.0)
                .padding_vert(6.0)
                .cursor(floem::style::CursorStyle::Pointer)
                .hover(|s| {
                    s.background(Color::from_rgb8(0x28, 0x28, 0x28))
                })
        })
    )
    .style(|s| {
        s.width_full()
            .justify_end()
    })
}

// EXACT Windsurf AI message from outerHTML
// Structure: flex min-w-0 grow flex-col gap-1.5
pub fn ai_message(text: String, show_feedback: bool) -> impl View {
    if show_feedback {
        container(
            v_stack((
                // Thought header
                container(
                    h_stack((
                        h_stack((
                            label(|| "Thought".to_string()),
                            label(|| "for 3s".to_string())
                                .style(|s| s.color(Color::from_rgb8(0xcc, 0xcc, 0xcc).multiply_alpha(0.7))),
                        ))
                        .style(|s| s.gap(4.0)),
                        label(|| "›".to_string())
                            .style(|s| s.color(Color::from_rgb8(0xcc, 0xcc, 0xcc).multiply_alpha(0.0))),
                    ))
                    .style(|s| s.items_center().gap(4.0).cursor(floem::style::CursorStyle::Pointer))
                )
                .style(|s| {
                    s.margin_vert(-4.0).padding_vert(4.0)
                        .font_size(12.0)
                        .color(Color::from_rgb8(0xcc, 0xcc, 0xcc).multiply_alpha(0.5))
                }),
                container(
                    label(move || text.clone())
                        .style(|s| {
                            s.font_size(13.0)
                                .line_height(1.6)
                                .color(Color::from_rgb8(0xcc, 0xcc, 0xcc))
                                .margin_bottom(0.0)
                                .margin_top(0.0)
                                .text_overflow(floem::style::TextOverflow::Wrap)
                        })
                )
                .style(|s| s.width_full().gap(4.0)),
                feedback_buttons(),
            ))
            .style(|s| s.flex_col().gap(6.0))
        )
        .style(|s| s.width_full().max_width_pct(90.0))
    } else {
        container(
            v_stack((
                // Thought header
                container(
                    h_stack((
                        h_stack((
                            label(|| "Thought".to_string()),
                            label(|| "for 3s".to_string())
                                .style(|s| s.color(Color::from_rgb8(0xcc, 0xcc, 0xcc).multiply_alpha(0.7))),
                        ))
                        .style(|s| s.gap(4.0)),
                        label(|| "›".to_string())
                            .style(|s| s.color(Color::from_rgb8(0xcc, 0xcc, 0xcc).multiply_alpha(0.0))),
                    ))
                    .style(|s| s.items_center().gap(4.0).cursor(floem::style::CursorStyle::Pointer))
                )
                .style(|s| {
                    s.margin_vert(-4.0).padding_vert(4.0)
                        .font_size(12.0)
                        .color(Color::from_rgb8(0xcc, 0xcc, 0xcc).multiply_alpha(0.5))
                }),
                container(
                    label(move || text.clone())
                        .style(|s| {
                            s.font_size(13.0)
                                .line_height(1.6)
                                .color(Color::from_rgb8(0xcc, 0xcc, 0xcc))
                                .margin_bottom(0.0)
                                .margin_top(0.0)
                                .text_overflow(floem::style::TextOverflow::Wrap)
                        })
                )
                .style(|s| s.width_full().gap(4.0)),
            ))
            .style(|s| s.flex_col().gap(6.0))
        )
        .style(|s| s.width_full().max_width_pct(90.0))
    }
}

// Feedback buttons: only thumbs up/down, left-aligned
fn feedback_buttons() -> impl View {
    h_stack((
        label(|| "👍".to_string())
            .style(|s| {
                s.font_size(14.0)
                    .color(Color::from_rgb8(0xcc, 0xcc, 0xcc).multiply_alpha(0.7))
                    .cursor(floem::style::CursorStyle::Pointer)
                    .hover(|s| s.color(Color::from_rgb8(0xcc, 0xcc, 0xcc)))
            }),
        label(|| "👎".to_string())
            .style(|s| {
                s.font_size(14.0)
                    .color(Color::from_rgb8(0xcc, 0xcc, 0xcc).multiply_alpha(0.7))
                    .cursor(floem::style::CursorStyle::Pointer)
                    .hover(|s| s.color(Color::from_rgb8(0xcc, 0xcc, 0xcc)))
            }),
    ))
    .style(|s| s.gap(12.0))
}

// EXACT Windsurf code block with language label and copy button
pub fn code_block(language: String, code: String) -> impl View {
    let code_copy = code.clone();
    
    container(
        v_stack((
            // Header bar with language and buttons
            container(
                h_stack((
                    // Language label (left)
                    label(move || language.clone())
                        .style(|s| {
                            s.font_size(13.0)
                                .color(Color::from_rgb8(0xcc, 0xcc, 0xcc).multiply_alpha(0.6))
                        }),
                    
                    // Buttons (@ and copy) - grouped on right
                    h_stack((
                        // @ button (context/reference)
                        container(
                            label(|| "@".to_string())
                                .style(|s| {
                                    s.font_size(12.0)
                                        .color(Color::from_rgb8(0xcc, 0xcc, 0xcc))
                                })
                        )
                        .on_click_stop(move |_| {
                            println!("[Code Block] Add to context");
                        })
                        .style(|s| {
                            s.padding(4.0)
                                .border_radius(4.0)
                                .cursor(floem::style::CursorStyle::Pointer)
                                .hover(|s| s.background(Color::from_rgb8(0x80, 0x80, 0x80).multiply_alpha(0.25)))
                        }),
                        
                        // Copy button
                        container(
                            label(|| "⧉".to_string())
                                .style(|s| {
                                    s.font_size(12.0)
                                        .color(Color::from_rgb8(0xcc, 0xcc, 0xcc))
                                })
                        )
                        .on_click_stop(move |_| {
                            println!("[Code Block] Copied: {}", code_copy);
                        })
                        .style(|s| {
                            s.padding(4.0)
                                .border_radius(4.0)
                                .cursor(floem::style::CursorStyle::Pointer)
                                .hover(|s| s.background(Color::from_rgb8(0x80, 0x80, 0x80).multiply_alpha(0.25)))
                        }),
                    ))
                    .style(|s| s.gap(2.0)),
                ))
                .style(|s| s.width_full().justify_between().items_center())
            )
            .style(|s| {
                s.width_full()
                    .padding_left(8.0)
                    .padding_right(4.0)
                    .padding_vert(6.0)
                    .background(Color::from_rgb8(0x73, 0x73, 0x73).multiply_alpha(0.3))
            }),
            
            // Code content
            container(
                container(
                    container(
                        label(move || code.clone())
                            .style(|s| {
                                s.font_size(13.0)
                                    .font_family("monospace".to_string())
                                    .line_height(1.6)
                                    .color(Color::from_rgb8(0xd4, 0xd4, 0xd4))
                                    .width_full()
                                    .max_width_full()
                                    .text_overflow(floem::style::TextOverflow::Wrap)
                            })
                    )
                    .style(|s| {
                        s.padding(16.0)
                            .width_full()
                            .max_width_full()
                    })
                )
                .style(|s| {
                    s.width_full()
                        .max_width_full()
                })
            )
            .style(|s| {
                s.width_full()
                    .background(Color::from_rgb8(0x1e, 0x1e, 0x1e))
            }),
        ))
    )
    .style(|s| {
        s.width_full()
            .border_radius(8.0)
            .background(Color::from_rgb8(0x73, 0x73, 0x73).multiply_alpha(0.3))
    })
}

// EXACT Windsurf file link from small.html
pub fn file_link(filename: String) -> impl View {
    let fname_click = filename.clone();
    
    h_stack((
        // Icon placeholder
        label(|| "📄".to_string())
            .style(|s| {
                s.flex_shrink(0.0)
                    .font_size(11.7)
            }),
        
        // Filename
        label(move || filename.clone())
            .style(|s| {
                s.font_size(11.7)
                    .font_family("monospace".to_string())
                    .line_height(1.0)
                    .color(Color::from_rgb8(0x3b, 0x8f, 0xd8))
                    .hover(|s| {
                        s.border_bottom(1.0)
                            .border_color(Color::from_rgb8(0x3b, 0x8f, 0xd8))
                    })
            }),
    ))
    .on_click_stop(move |_| {
        println!("[File] Open: {}", fname_click);
    })
    .style(|s| {
        s.items_baseline()
            .gap(2.0)
            .cursor(floem::style::CursorStyle::Pointer)
    })
}

// EXACT Windsurf input bar from demo
pub fn input_bar<F>(
    input_value: RwSignal<String>,
    on_send: F,
    sending_disabled: bool,
    selected_model: RwSignal<String>,
    selected_mode: RwSignal<String>,
) -> impl View 
where
    F: Fn() + 'static + Clone,
{
    use floem::views::text_input;
    let _ = selected_model;
    let _ = selected_mode;
    
    container(
        v_stack((
            // Text input area
            container(
                v_stack((
                    // Input field with placeholder
                    container(
                        {
                            let on_send = on_send.clone();
                            text_input(input_value)
                                .placeholder("Ask anything (Ctrl+L)".to_string())
                                .on_event_cont(floem::event::EventListener::KeyDown, move |e| {
                                    if let floem::event::Event::KeyDown(key) = e {
                                        if key.key.logical_key == floem::keyboard::Key::Named(floem::keyboard::NamedKey::Enter) 
                                            && !key.modifiers.shift() {
                                            on_send();
                                        }
                                    }
                                })
                        }
                            .style(|s| {
                                s.width_full()
                                    .min_height(32.0)
                                    .padding(0.0)
                                    .background(Color::TRANSPARENT)
                                    .border(0.0)
                                    .color(Color::from_rgb8(0xcc, 0xcc, 0xcc))
                            })
                    )
                    .style(|s| {
                        s.width_full()
                            .padding_left(3.0)
                            .padding_top(1.0)
                            .padding_bottom(4.0)
                    }),
                ))
                .style(|s| s.width_full())
            )
            .style(|s| s.width_full()),
            
            // Bottom toolbar
            h_stack((
                // Plus button
                container(
                    label(|| "+".to_string())
                        .style(|s| {
                            s.font_size(16.0)
                                .color(Color::from_rgb8(0xcc, 0xcc, 0xcc))
                        })
                )
                .on_click_stop(move |_| {
                    println!("[Input] Plus button clicked");
                })
                .style(|s| {
                    s.padding(2.0)
                        .padding_left(4.0)
                        .padding_right(4.0)
                        .border_radius(4.0)
                        .cursor(floem::style::CursorStyle::Pointer)
                        .hover(|s| s.background(Color::from_rgb8(0x80, 0x80, 0x80).multiply_alpha(0.1)))
                }),
                
                // Spacer
                container(label(|| "".to_string()))
                    .style(|s| s.flex_grow(1.0)),
                
                // Right buttons group
                h_stack((
                    // Microphone button with SVG
                    container(
                        svg(|| r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 19v3"/><path d="M19 10v2a7 7 0 0 1-14 0v-2"/><rect x="9" y="2" width="6" height="13" rx="3"/></svg>"#.to_string())
                            .style(|s| {
                                s.width(14.0)
                                    .height(14.0)
                                    .color(Color::from_rgb8(0xcc, 0xcc, 0xcc))
                            })
                    )
                    .on_click_stop(move |_| {
                        println!("[Input] Mic button clicked");
                    })
                    .style(|s| {
                        s.padding(2.0)
                            .border_radius(4.0)
                            .cursor(floem::style::CursorStyle::Pointer)
                            .hover(|s| s.background(Color::from_rgb8(0x80, 0x80, 0x80).multiply_alpha(0.2)))
                    }),
                    
                    // Send button
                    container(
                        label(|| "↑".to_string())
                            .style(|s| {
                                s.color(Color::from_rgb8(0x1e, 0x1e, 0x1e))
                                    .font_size(12.0)
                                    .font_weight(floem::text::Weight::BOLD)
                            })
                    )
                    .on_click_stop({
                        let on_send = on_send.clone();
                        move |_| {
                            if !sending_disabled {
                                on_send();
                            }
                        }
                    })
                    .style(move |s| {
                        let bg_color = if sending_disabled {
                            Color::from_rgb8(0xcc, 0xcc, 0xcc).multiply_alpha(0.5)
                        } else {
                            Color::from_rgb8(0xcc, 0xcc, 0xcc)
                        };
                        s.width(20.0)
                            .height(20.0)
                            .border_radius(10.0)
                            .background(bg_color)
                            .justify_center()
                            .items_center()
                            .cursor(if sending_disabled { 
                                floem::style::CursorStyle::Default 
                            } else { 
                                floem::style::CursorStyle::Pointer 
                            })
                    }),
                ))
                .style(|s| s.items_center().gap(4.0)),
            ))
            .style(|s| s.width_full().items_center().justify_between().gap(6.0)),
        ))
        .style(|s| s.flex_col())
    )
    .style(|s| {
        s.width_full()
            .padding(8.0)
            .padding_horiz(12.0)
            .background(Color::from_rgb8(0x1f, 0x1f, 0x1f))
            .border(1.0)
            .border_color(Color::from_rgb8(0x33, 0x33, 0x33))
            .border_radius(12.0)
            .margin(16.0)
    })
}

                    .hover(|s| s.background(Color::from_rgb8(0x80, 0x80, 0x80).multiply_alpha(0.1)))
            }),
        ))
    )
    .style(|s| s.position(floem::style::Position::Relative))
