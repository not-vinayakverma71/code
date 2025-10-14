// Windsurf Chat Demo - REAL conversation from small.html
// Exact messages extracted from actual Windsurf chat

use floem::{
    peniko::Color,
    reactive::{create_rw_signal, SignalGet, SignalUpdate},
    views::{
        container, h_stack, label, scroll, svg, v_stack, Decorators, text_input,
    },
    Application, View,
};

fn main() {
    Application::new()
        .window(
            |_| {
                windsurf_chat_view()
            },
            None,
        )
        .run();
}

fn windsurf_chat_view() -> impl View {
    v_stack((
        // Header
        header_bar(),

        // Messages area - REAL Windsurf conversation
        scroll(
            container(
                v_stack((
                    v_stack((
                        user_msg("tell me about this repo"),
                        ai_msg("I'll analyze the Lapce repository structure for you."),
                        ai_msg_with_files("I'll read the following files:"),
                        ai_reading_file("windsurf_chat_demo.rs", "#L60-64"),
                        ai_reading_file("write.rs", ""),
                        ai_msg("Based on my analysis, here's what I found:"),
                        ai_section_with_files("Repository Focus", "Lapce is a Rust-based code editor with UI built in Floem. Core functionality spans multiple crates like lapce-ai/, lapce-app/, lapce-core/, lapce-proxy/, and lapce-rpc/, each targeting distinct subsystems.", vec!["README.md"]),
                        ai_section_with_files("AI Integration", "lapce-ai/ houses AI tooling, including IPC messaging and tool abstraction layers. Supporting markdown references detail precise UI specifications derived from real DOM captures.", vec!["lapce-ai/src/ipc_messages.rs", "lapce-ai/src/core/tools/traits.rs", "WINDSURF_INPUT_EXACT.md"]),
                    ))
                    .style(|s| s.flex_col().gap(12.0)),
                    
                    v_stack((
                        ai_section_with_files("UI Documentation Backbone", "Numerous markdown specs guide UI/UX implementation, ensuring fidelity to Windsurf-inspired designs.", vec!["WINDSURF_FULL_UI_STRUCTURE.md", "VSCODE_INPUT_STYLING.md", "CLEAN_AI_CHAT_UI.md"]),
                        ai_msg("Let me run a search to find related files:"),
                        terminal_output("grep -r 'Windsurf' . --include='*.md'", vec![
                            "WINDSURF_FULL_UI_STRUCTURE.md (3 matches)",
                            "WINDSURF_INPUT_EXACT.md (3 matches)",
                            "WINDSURF_UI_COMPONENTS.md (2 matches)",
                            "WINDSURF_INPUT_CHANGES.md (1 match)",
                            "Plus 2 Codex documentation files",
                        ]),
                        ai_semantic_search("icon_size", "lapce-app/src/config", "*.rs", vec![
                            ("lapce-app/src/config/ui.rs", 20),
                            ("lapce-app/src/config/ui.rs", 124),
                            ("lapce-app/src/config/ui.rs", 125),
                            ("lapce-app/src/config/ui.rs", 128),
                        ]),
                        ai_msg("Here's how to run the updated demo:"),
                        ai_code_block("bash", "killall lapce 2>/dev/null\ncd /home/verma/lapce\n./target/release/lapce ."),
                        ai_msg("Here's the Rust code for the new component:"),
                        ai_code_block("rust", "fn ai_code_block(language: &str, code: &str) -> impl View {\n    container(\n        v_stack((\n            label(language),\n            label(code),\n        ))\n    )\n}"),
                        ai_msg("And here's a Python example for testing:"),
                        ai_code_block("python", "def process_files(directory):\n    for file in os.listdir(directory):\n        if file.endswith('.rs'):\n            print(f'Processing {file}')"),
                        ai_msg("I'll update the windsurf_chat_demo.rs file to improve the terminal styling:"),
                        file_edit_item("windsurf_chat_demo.rs", 19, 19),
                        ai_error_editing_file("windsurf_chat_demo.rs"),
                        ai_todo_msg(vec![
                            "Finalize Windsurf Input Implementation",
                            "Implement Advanced Button States",
                            "Polish Interaction Experience",
                            "Theme Synchronization Audits",
                            "Automated Testing Coverage",
                            "Performance Monitoring Review",
                        ]),
                        ai_status_msg_final(vec![
                            "Project surface analyzed.",
                            "Structured TODO drafted for next steps.",
                        ]),
                    ))
                    .style(|s| s.flex_col().gap(12.0)),
                ))
                .style(|s| s.flex_col().gap(12.0))
            )
            .style(|s| s.padding(16.0).max_width_full())
        )
        .style(|s| s.flex_grow(1.0).max_width_full()),

        // Input area
        input_bar(),
    ))
    .style(|s| {
        s.width_full()
            .height_full()
            .background(Color::from_rgb8(0x1e, 0x1e, 0x1e))
    })
}

fn header_bar() -> impl View {
    container(
        label(|| "Windsurf Chat - Real Conversation".to_string())
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
    })
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

// EXACT Windsurf user message from outerHTML
// Right-aligned: flex justify-end, rounded-[8px] border bg-ide-input-background px-[9px] py-[6px]
fn user_msg(text: &str) -> impl View {
    let msg = text.to_string();
    
    container(
        container(
            label(move || msg.clone())
                .style(|s| {
                    s.font_size(12.0)  // [&_p]:text-[12px]
                        .color(Color::from_rgb8(0xcc, 0xcc, 0xcc))
                        .line_height(1.5)
                        .text_overflow(floem::style::TextOverflow::Wrap)
                })
        )
        .style(|s| {
            s.border_radius(8.0)  // rounded-[8px]
                .border(1.0)
                .border_color(Color::from_rgb8(0x7d, 0x7d, 0x7d).multiply_alpha(0.125))  // border-[#7d7d7d20]
                .background(Color::from_rgb8(0x31, 0x31, 0x31))  // bg-ide-input-background
                .padding_horiz(9.0)  // px-[9px]
                .padding_vert(6.0)  // py-[6px]
                .cursor(floem::style::CursorStyle::Pointer)
                .hover(|s| {
                    s.background(Color::from_rgb8(0x28, 0x28, 0x28))
                })
        })
    )
    .style(|s| {
        s.width_full()
            .justify_end()  // User message right-aligned
    })
}

// AI message WITHOUT feedback (for intermediate messages)
fn ai_msg(text: &str) -> impl View {
    ai_msg_internal(text, false)
}

// AI message WITH feedback (for final message)
fn ai_msg_final(text: &str) -> impl View {
    ai_msg_internal(text, true)
}

// EXACT Windsurf AI message from outerHTML
// Structure: flex min-w-0 grow flex-col gap-1.5
fn ai_msg_internal(text: &str, show_feedback: bool) -> impl View {
    let msg = text.to_string();
    
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
                    label(move || msg.clone())
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
                    label(move || msg.clone())
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

fn ai_section(title: &str, content: &str) -> impl View {
    let title_str = title.to_string();
    let content_str = content.to_string();
    
    container(
        v_stack((
            // Thought header
            container(
                h_stack((
                    h_stack((
                        label(|| "Thought".to_string()),
                        label(|| "for 2s".to_string())
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
            
            // Content
            container(
                v_stack((
                    label(move || format!("• {}", title_str.clone()))
                        .style(|s| {
                            s.font_size(13.0)
                                .font_weight(floem::text::Weight::BOLD)
                                .line_height(1.6)
                                .color(Color::from_rgb8(0xcc, 0xcc, 0xcc))
                        }),
                    label(move || content_str.clone())
                        .style(|s| {
                            s.font_size(13.0)
                                .line_height(1.6)
                                .color(Color::from_rgb8(0xcc, 0xcc, 0xcc))
                                .margin_left(12.0)
                                .margin_top(4.0)
                        }),
                ))
            )
            .style(|s| s.width_full().gap(4.0)),
        ))
        .style(|s| s.flex_col().gap(6.0))
    )
    .style(|s| s.width_full().max_width_pct(90.0))
}

fn ai_section_with_files(title: &str, content: &str, files: Vec<&str>) -> impl View {
    let title_str = title.to_string();
    let content_str = content.to_string();
    let f0 = files.get(0).unwrap_or(&"").to_string();
    let f1 = files.get(1).unwrap_or(&"").to_string();
    let f2 = files.get(2).unwrap_or(&"").to_string();
    let has_f0 = !f0.is_empty();
    let has_f1 = !f1.is_empty();
    let has_f2 = !f2.is_empty();
    
    container(
        v_stack((
            // Thought header
            container(
                h_stack((
                    h_stack((
                        label(|| "Thought".to_string()),
                        label(|| "for 2s".to_string())
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
            
            // Content
            container(
                v_stack((
                    label(move || format!("• {}", title_str.clone()))
                        .style(|s| {
                            s.font_size(13.0)
                                .font_weight(floem::text::Weight::BOLD)
                                .line_height(1.6)
                                .color(Color::from_rgb8(0xcc, 0xcc, 0xcc))
                        }),
                    label(move || content_str.clone())
                        .style(|s| {
                            s.font_size(13.0)
                                .line_height(1.6)
                                .color(Color::from_rgb8(0xcc, 0xcc, 0xcc))
                                .margin_left(12.0)
                                .margin_top(4.0)
                        }),
                    container(
                        v_stack((
                            file_link_inline_owned(f0.clone())
                                .style(move |s| if has_f0 { s } else { s.display(floem::style::Display::None) }),
                            file_link_inline_owned(f1.clone())
                                .style(move |s| if has_f1 { s } else { s.display(floem::style::Display::None) }),
                            file_link_inline_owned(f2.clone())
                                .style(move |s| if has_f2 { s } else { s.display(floem::style::Display::None) }),
                        ))
                    )
                    .style(|s| s.flex_col().gap(6.0).margin_left(12.0).margin_top(8.0)),
                ))
            )
            .style(|s| s.width_full().gap(4.0)),
        ))
        .style(|s| s.flex_col().gap(6.0))
    )
    .style(|s| s.width_full().max_width_pct(90.0))
}

// AI message with inline file links (Windsurf style)
fn ai_msg_with_files(intro: &str) -> impl View {
    let intro_text = intro.to_string();
    
    container(
        v_stack((
            // Thought header
            container(
                h_stack((
                    h_stack((
                        label(|| "Thought".to_string()),
                        label(|| "for 1s".to_string())
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
            
            // Content
            container(
                v_stack((
                    label(move || intro_text.clone())
                        .style(|s| {
                            s.font_size(13.0)
                                .line_height(1.6)
                                .color(Color::from_rgb8(0xcc, 0xcc, 0xcc))
                        }),
                    v_stack((
                        file_link_inline("README.md"),
                        file_link_inline("WINDSURF_FULL_UI_STRUCTURE.md"),
                        file_link_inline("WINDSURF_INPUT_EXACT.md"),
                    ))
                    .style(|s| s.flex_col().gap(4.0).margin_top(8.0)),
                ))
            )
            .style(|s| s.width_full().gap(4.0)),
        ))
        .style(|s| s.flex_col().gap(6.0))
    )
    .style(|s| s.width_full().max_width_pct(90.0))
}

// EXACT Windsurf file link from small.html line 7643
// Classes: inline-flex items-baseline gap-0.5 text-[0.9em] cursor-pointer hover:underline font-mono leading-[1rem] text-ide-link-color
fn file_link_inline(filename: &str) -> impl View {
    file_link_inline_owned(filename.to_string())
}

fn file_link_inline_owned(filename: String) -> impl View {
    let fname = filename.clone();
    let fname_click = filename.clone();
    
    h_stack((
        // Icon placeholder (flex-shrink-0)
        label(|| "📄".to_string())
            .style(|s| {
                s.flex_shrink(0.0)
                    .font_size(11.7)  // 0.9em * 13px = 11.7px
            }),
        
        // Filename
        label(move || fname.clone())
            .style(|s| {
                s.font_size(11.7)  // text-[0.9em] = 0.9 * 13px
                    .font_family("monospace".to_string())  // font-mono
                    .line_height(1.0)  // leading-[1rem]
                    .color(Color::from_rgb8(0x3b, 0x8f, 0xd8))  // darker blue #3b8fd8
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
        s.items_baseline()  // items-baseline
            .gap(2.0)  // gap-0.5 = 0.125rem = 2px
            .cursor(floem::style::CursorStyle::Pointer)
    })
}

// EXACT Windsurf file edit item (when AI edits a file) - complete with Thought header
fn file_edit_item(filename: &str, additions: i32, deletions: i32) -> impl View {
    let fname = filename.to_string();
    let fname_click = filename.to_string();
    
    container(
        v_stack((
            // Thought header
            container(
                h_stack((
                    h_stack((
                        label(|| "Thought".to_string()),
                        label(|| "for 1s".to_string())
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
            
            // File edit box
            container(
                container(
                    h_stack((
                        // Left side: icon + filename
                        h_stack((
                            // File icon placeholder
                            label(|| "📄".to_string())
                                .style(|s| {
                                    s.flex_shrink(0.0)
                                        .font_size(14.0)
                                        .line_height(1.0)
                                }),
                            
                            // Filename
                            label(move || fname.clone())
                                .style(|s| {
                                    s.font_size(13.0)
                                        .font_weight(floem::text::Weight::LIGHT)
                                        .color(Color::from_rgb8(0xcc, 0xcc, 0xcc))
                                }),
                        ))
                        .style(|s| s.flex_grow(1.0).gap(8.0).items_center()),
                        
                        // Right side: +/- counts
                        h_stack((
                            label(move || format!("+{}", additions))
                                .style(|s| {
                                    s.font_size(13.0)
                                        .font_weight(floem::text::Weight::MEDIUM)
                                        .color(Color::from_rgb8(0x4d, 0x7c, 0x3f))  // darker green #4d7c3f
                                }),
                            label(move || format!("-{}", deletions))
                                .style(|s| {
                                    s.font_size(13.0)
                                        .font_weight(floem::text::Weight::MEDIUM)
                                        .color(Color::from_rgb8(0xd3, 0x2f, 0x2f))  // darker red #d32f2f
                                }),
                        ))
                        .style(|s| s.gap(4.0).flex_shrink(0.0).margin_left(8.0)),
                    ))
                    .on_click_stop(move |_| {
                        println!("[File Edit] Open: {}", fname_click);
                    })
                    .style(|s| {
                        s.width_full()
                            .padding_horiz(8.0)
                            .padding_vert(6.0)  // py-1.5
                            .border_radius(8.0)  // rounded-lg
                            .items_center()
                            .cursor(floem::style::CursorStyle::Pointer)
                            .hover(|s| {
                                s.background(Color::from_rgb8(0x73, 0x73, 0x73).multiply_alpha(0.15))  // hover darker
                            })
                    })
                )
                .style(|s| {
                    s.width_full()
                        .background(Color::from_rgb8(0x73, 0x73, 0x73).multiply_alpha(0.1))  // bg-neutral-500/10
                        .border_radius(8.0)
                        .padding(4.0)
                })
            )
            .style(|s| s.width_full()),
        ))
        .style(|s| s.flex_col().gap(6.0))
    )
    .style(|s| s.width_full().max_width_pct(90.0))
}

// EXACT Windsurf AI reading file indicator with collapsible Thought section
fn ai_reading_file(filename: &str, line_range: &str) -> impl View {
    let fname = filename.to_string();
    let fname_click = filename.to_string();
    let fname_display = filename.to_string();
    let fname_thought = filename.to_string();
    let range = line_range.to_string();
    let range_click = line_range.to_string();
    let range_display = line_range.to_string();
    let range_thought = line_range.to_string();
    
    let is_expanded = create_rw_signal(false);  // Start collapsed
    
    container(
        v_stack((
            // Thought header (clickable to expand/collapse)
            container(
                h_stack((
                    label(|| "Thought".to_string()),
                    label(|| "for 9s".to_string())
                        .style(|s| s.color(Color::from_rgb8(0xcc, 0xcc, 0xcc).multiply_alpha(0.7))),
                ))
                .style(|s| s.gap(4.0))
                .on_click_stop(move |_| {
                    let was_expanded = is_expanded.get();
                    is_expanded.update(|v| *v = !*v);
                    println!("[Thought] Clicked! Was: {}, Now: {}", was_expanded, !was_expanded);
                })
            )
            .style(|s| {
                s.margin_vert(-4.0).padding_vert(4.0)
                    .font_size(12.0)
                    .color(Color::from_rgb8(0xcc, 0xcc, 0xcc).multiply_alpha(0.5))
                    .cursor(floem::style::CursorStyle::Pointer)
            }),
            
            // Thought content (collapsible reasoning text ONLY)
            container(
                label(move || {
                    if range_thought.is_empty() {
                        format!("I need to read the entire {} file to understand its structure and content.", fname_thought.clone())
                    } else {
                        format!("I'll examine lines {} of {} to see the specific implementation details.", range_thought.clone(), fname_thought.clone())
                    }
                })
                .style(|s| {
                    s.font_size(13.0)
                        .color(Color::from_rgb8(0xcc, 0xcc, 0xcc).multiply_alpha(0.5))
                        .line_height(1.6)
                })
            )
            .style(move |s| {
                let mut style = s.margin_left(8.0).padding_vert(4.0);
                if !is_expanded.get() {
                    style = style.hide();
                }
                style
            }),
            
            // Read indicator (ALWAYS VISIBLE - outside thought)
            h_stack((
                label(|| "Read".to_string())
                    .style(|s| {
                        s.font_size(13.0)
                            .color(Color::from_rgb8(0xcc, 0xcc, 0xcc).multiply_alpha(0.5))
                    }),
                
                label(move || {
                    if range_display.is_empty() {
                        fname_display.clone()
                    } else {
                        format!("{} {}", fname_display.clone(), range_display.clone())
                    }
                })
                .on_click_stop(move |_| {
                    if range_click.is_empty() {
                        println!("[Read File] Open full file: {}", fname_click);
                    } else {
                        println!("[Read File] Open file with range: {} {}", fname_click, range_click);
                    }
                })
                .style(|s| {
                    s.font_size(13.0)
                        .color(Color::from_rgb8(0xcc, 0xcc, 0xcc).multiply_alpha(0.7))
                        .cursor(floem::style::CursorStyle::Pointer)
                        .hover(|s| s.color(Color::from_rgb8(0xcc, 0xcc, 0xcc)))
                }),
            ))
            .style(|s| s.gap(4.0).items_center().padding_vert(4.0)),
        ))
        .style(|s| s.flex_col().gap(6.0))
    )
    .style(|s| s.width_full().max_width_pct(90.0))
}

// EXACT Windsurf semantic search indicator with collapsible results
fn ai_semantic_search(query: &str, path: &str, file_pattern: &str, results: Vec<(&str, usize)>) -> impl View {
    let search_text = format!("{} in {} ({}) ({})", query, path, file_pattern, results.len());
    let is_expanded = create_rw_signal(false);
    
    // Clone for each closure
    let results_display: Vec<(String, usize)> = results.iter()
        .map(|(f, l)| (f.to_string(), *l))
        .collect();
    let r0 = results_display.clone();
    let r0_click = results_display.clone();
    let r1 = results_display.clone();
    let r1_click = results_display.clone();
    let r2 = results_display.clone();
    let r2_click = results_display.clone();
    let r3 = results_display.clone();
    let r3_click = results_display.clone();
    
    container(
        v_stack((
            // Search header (clickable to expand/collapse)
            container(
                h_stack((
                    label(|| "Searched".to_string()),
                    label(move || search_text.clone())
                        .style(|s| s.color(Color::from_rgb8(0xcc, 0xcc, 0xcc).multiply_alpha(0.7))),
                ))
                .style(|s| s.gap(6.0))
                .on_click_stop(move |_| {
                    let was_expanded = is_expanded.get();
                    is_expanded.update(|v| *v = !*v);
                    println!("[Semantic Search] Clicked! Was: {}, Now: {}", was_expanded, !was_expanded);
                })
            )
            .style(|s| {
                s.margin_vert(-4.0).padding_vert(4.0)
                    .font_size(13.0)
                    .color(Color::from_rgb8(0xcc, 0xcc, 0xcc).multiply_alpha(0.5))
                    .cursor(floem::style::CursorStyle::Pointer)
            }),
            
            // Search results (collapsible)
            container(
                v_stack((
                    label(move || format!("{}:{}", r0[0].0.clone(), r0[0].1))
                        .on_click_stop(move |_| {
                            println!("[Search Result] Open: {}:{}", r0_click[0].0, r0_click[0].1);
                        })
                        .style(|s| {
                            s.font_size(13.0)
                                .font_family("monospace".to_string())
                                .color(Color::from_rgb8(0x9c, 0xda, 0xfe))
                                .cursor(floem::style::CursorStyle::Pointer)
                                .hover(|s| s.color(Color::from_rgb8(0xb0, 0xe5, 0xff)))
                        }),
                    label(move || format!("{}:{}", r1[1].0.clone(), r1[1].1))
                        .on_click_stop(move |_| {
                            println!("[Search Result] Open: {}:{}", r1_click[1].0, r1_click[1].1);
                        })
                        .style(|s| {
                            s.font_size(13.0)
                                .font_family("monospace".to_string())
                                .color(Color::from_rgb8(0x9c, 0xda, 0xfe))
                                .cursor(floem::style::CursorStyle::Pointer)
                                .hover(|s| s.color(Color::from_rgb8(0xb0, 0xe5, 0xff)))
                        }),
                    label(move || format!("{}:{}", r2[2].0.clone(), r2[2].1))
                        .on_click_stop(move |_| {
                            println!("[Search Result] Open: {}:{}", r2_click[2].0, r2_click[2].1);
                        })
                        .style(|s| {
                            s.font_size(13.0)
                                .font_family("monospace".to_string())
                                .color(Color::from_rgb8(0x9c, 0xda, 0xfe))
                                .cursor(floem::style::CursorStyle::Pointer)
                                .hover(|s| s.color(Color::from_rgb8(0xb0, 0xe5, 0xff)))
                        }),
                    label(move || format!("{}:{}", r3[3].0.clone(), r3[3].1))
                        .on_click_stop(move |_| {
                            println!("[Search Result] Open: {}:{}", r3_click[3].0, r3_click[3].1);
                        })
                        .style(|s| {
                            s.font_size(13.0)
                                .font_family("monospace".to_string())
                                .color(Color::from_rgb8(0x9c, 0xda, 0xfe))
                                .cursor(floem::style::CursorStyle::Pointer)
                                .hover(|s| s.color(Color::from_rgb8(0xb0, 0xe5, 0xff)))
                        }),
                ))
                .style(|s| s.flex_col().gap(4.0))
            )
            .style(move |s| {
                let mut style = s.margin_left(8.0).padding_vert(4.0);
                if !is_expanded.get() {
                    style = style.hide();
                }
                style
            }),
        ))
        .style(|s| s.flex_col().gap(6.0))
    )
    .style(|s| s.width_full().max_width_pct(90.0))
}

// EXACT Windsurf code block with language label and copy button
fn ai_code_block(language: &str, code: &str) -> impl View {
    let lang = language.to_string();
    let code_text = code.to_string();
    let code_copy = code.to_string();
    
    container(
        v_stack((
            // Header bar with language and buttons
            container(
                h_stack((
                    // Language label (left)
                    label(move || lang.clone())
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
                            label(|| "⧉".to_string())  // Copy icon
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
                    .background(Color::from_rgb8(0x73, 0x73, 0x73).multiply_alpha(0.3))  // Darker grey header
            }),
            
            // Code content
            container(
                container(
                    container(
                        label(move || code_text.clone())
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
                    .max_width_full()
                    .background(Color::from_rgb8(0x0d, 0x0d, 0x0d))  // Darker code background - distinct from chat
            }),
        ))
        .style(|s| s.flex_col().gap(0.0))  // NO gap between header and code
    )
    .style(|s| {
        s.width_full()
            .max_width_pct(90.0)
            .border_radius(6.0)  // Rounded corners
            .margin_vert(8.0)
    })
}

// EXACT Windsurf error while editing file indicator
fn ai_error_editing_file(filename: &str) -> impl View {
    let fname = filename.to_string();
    let fname_click = filename.to_string();
    
    container(
        h_stack((
            // Left side: "Error while editing" + filename
            h_stack((
                label(|| "Error while editing".to_string())
                    .style(|s| {
                        s.font_size(13.0)
                            .color(Color::from_rgb8(0xff, 0x63, 0x47))  // Red for error
                    }),
                
                // Filename (clickable with file icon)
                label(move || fname.clone())
                    .on_click_stop(move |_| {
                        println!("[Error File] Open: {}", fname_click);
                    })
                    .style(|s| {
                        s.font_size(13.0)
                            .font_family("monospace".to_string())
                            .color(Color::from_rgb8(0x9c, 0xda, 0xfe))  // Link color
                            .cursor(floem::style::CursorStyle::Pointer)
                            .hover(|s| {
                                s.color(Color::from_rgb8(0xb0, 0xe5, 0xff))  // Brighter on hover
                            })
                    }),
            ))
            .style(|s| s.gap(6.0).items_center()),
            
            // Right side: chevron (hidden by default, shown on hover)
            label(|| "›".to_string())
                .style(|s| {
                    s.font_size(12.0)
                        .color(Color::from_rgb8(0xcc, 0xcc, 0xcc).multiply_alpha(0.0))
                }),
        ))
        .style(|s| s.items_center().gap(4.0))
    )
    .style(|s| {
        s.width_full()
            .max_width_pct(90.0)
            .padding_vert(4.0)
            .cursor(floem::style::CursorStyle::Pointer)
    })
}

// EXACT Windsurf terminal from HTML: rounded box with header and output sections
fn terminal_output(command: &str, output_lines: Vec<&str>) -> impl View {
    let cmd = command.to_string();
    let output_text = output_lines.join("\n");
    
    container(
        v_stack((
            // Thought header
            container(
                h_stack((
                    h_stack((
                        label(|| "Thought".to_string()),
                        label(|| "for 1s".to_string())
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
            
            // Terminal box: bg-neutral-500/10 rounded
            container(
                v_stack((
                    // Header: directory + command + buttons (rounded-t px-2 py-2)
                    container(
                        h_stack((
                            // Command line with directory (inline like HTML <pre> with <span>s)
                            h_stack((
                                label(|| "lapce-app".to_string())
                                    .style(|s| {
                                        s.font_family("monospace".to_string())
                                            .font_size(11.0)  // text-xs
                                            .color(Color::from_rgb8(0xcc, 0xcc, 0xcc).multiply_alpha(0.5))  // opacity-50
                                    }),
                                label(|| "$ ".to_string())
                                    .style(|s| {
                                        s.font_family("monospace".to_string())
                                            .font_size(11.0)
                                            .color(Color::from_rgb8(0xcc, 0xcc, 0xcc).multiply_alpha(0.5))  // opacity-50
                                    }),
                                label(move || cmd.clone())
                                    .style(|s| {
                                        s.font_family("monospace".to_string())
                                            .font_size(11.0)
                                            .color(Color::from_rgb8(0xcc, 0xcc, 0xcc))  // normal opacity
                                    }),
                            ))
                            .style(|s| s.flex_grow(1.0).items_center()),
                            
                            // Buttons: terminal icon, copy icon, stop icon (Unicode approximations)
                            h_stack((
                                // Terminal icon: ▣ (square with horizontal lines)
                                container(
                                    label(|| "▣".to_string())
                                )
                                .style(|s| {
                                    s.font_size(12.0)  // size-3
                                        .color(Color::from_rgb8(0xcc, 0xcc, 0xcc))
                                        .cursor(floem::style::CursorStyle::Pointer)
                                        .padding(1.0)
                                }),
                                
                                // Copy icon: ⧉ (two overlapping squares)
                                container(
                                    label(|| "⧉".to_string())
                                )
                                .style(|s| {
                                    s.font_size(12.0)
                                        .color(Color::from_rgb8(0xcc, 0xcc, 0xcc))
                                        .cursor(floem::style::CursorStyle::Pointer)
                                        .padding(1.0)
                                }),
                                
                                // Stop icon: ⊙ (circle with dot/square inside)
                                container(
                                    label(|| "⊙".to_string())
                                )
                                .style(|s| {
                                    s.font_size(12.0)
                                        .color(Color::from_rgb8(0xcc, 0xcc, 0xcc))
                                        .cursor(floem::style::CursorStyle::Pointer)
                                        .padding(1.0)
                                }),
                            ))
                            .style(|s| s.gap(4.0)),
                        ))
                    )
                    .style(|s| {
                        s.padding_horiz(8.0)
                            .padding_vert(8.0)
                            .items_center()
                            .justify_between()
                    }),
                    
                    // Output section: rounded-b bg-neutral-500/15
                    container(
                        container(
                            v_stack((
                                label(move || output_text.clone())
                                    .style(|s| {
                                        s.font_family("monospace".to_string())
                                            .font_size(11.0)  // text-xs
                                            .color(Color::from_rgb8(0xcc, 0xcc, 0xcc))
                                            .line_height(1.5)
                                    }),
                                label(|| "Exit Code 0".to_string())
                                    .style(|s| {
                                        s.font_family("monospace".to_string())
                                            .font_size(11.0)
                                            .color(Color::from_rgb8(0xcc, 0xcc, 0xcc).multiply_alpha(0.5))
                                            .margin_top(4.0)
                                    }),
                            ))
                        )
                        .style(|s| {
                            s.padding_horiz(8.0)
                                .padding_vert(4.0)
                        })
                    )
                    .style(|s| {
                        s.background(Color::from_rgb8(0x73, 0x73, 0x73).multiply_alpha(0.15))  // bg-neutral-500/15
                    }),
                ))
            )
            .style(|s| {
                s.width_full()
                    .background(Color::from_rgb8(0x73, 0x73, 0x73).multiply_alpha(0.1))  // bg-neutral-500/10
                    .border_radius(6.0)
            }),
        ))
        .style(|s| s.flex_col().gap(6.0))
    )
    .style(|s| s.width_full().max_width_pct(90.0))
}

// EXACT Windsurf TODO with Task progress header and collapsible list
fn ai_todo_msg(items: Vec<&str>) -> impl View {
    let todo_items: Vec<String> = items.iter().map(|s| s.to_string()).collect();
    let todo0 = todo_items.clone();
    let todo1 = todo_items.clone();
    let todo2 = todo_items.clone();
    let todo3 = todo_items.clone();
    let todo4 = todo_items.clone();
    let todo5 = todo_items.clone();
    
    let is_expanded = create_rw_signal(false);  // Start collapsed
    let completed_count = 3;  // 3 completed (0, 1, 2)
    let total_count = items.len();
    
    container(
        v_stack((
            // Task progress header (clickable to expand/collapse)
            container(
                label(move || format!("Task {} of {} done", completed_count, total_count))
                    .on_click_stop(move |_| {
                        let was_expanded = is_expanded.get();
                        is_expanded.update(|v| *v = !*v);
                        println!("[Task] Clicked! Was: {}, Now: {}", was_expanded, !was_expanded);
                    })
            )
            .style(|s| {
                s.margin_vert(-4.0).padding_vert(4.0)
                    .font_size(13.0)
                    .color(Color::from_rgb8(0xcc, 0xcc, 0xcc).multiply_alpha(0.5))
                    .cursor(floem::style::CursorStyle::Pointer)
            }),
            
            // TODO content (collapsible, dimmed at 50% opacity)
            container(
                container(
                    v_stack((
                        todo_item_windsurf(todo0.get(0).cloned().unwrap_or_default(), TodoState::Completed),
                        todo_item_windsurf(todo1.get(1).cloned().unwrap_or_default(), TodoState::Completed),
                        todo_item_windsurf(todo2.get(2).cloned().unwrap_or_default(), TodoState::Completed),
                        todo_item_windsurf_numbered(4, todo3.get(3).cloned().unwrap_or_default()),
                        todo_item_windsurf(todo4.get(4).cloned().unwrap_or_default(), TodoState::Pending),
                        todo_item_windsurf(todo5.get(5).cloned().unwrap_or_default(), TodoState::Pending),
                    ))
                    .style(|s| s.flex_col().gap(10.0))
                )
                .style(move |s| {
                    s.cursor(floem::style::CursorStyle::Pointer)
                        .flex_col()
                        .gap(4.0)
                        .background(Color::from_rgb8(0x73, 0x73, 0x73).multiply_alpha(0.1))
                        .padding_vert(6.0)
                        .padding_horiz(8.0)
                        .border_radius(8.0)
                })
            )
            .style(move |s| {
                let mut style = s.width_full().gap(4.0);
                // Hide when collapsed
                if !is_expanded.get() {
                    style = style.hide();
                }
                style
            }),
        ))
        .style(|s| s.flex_col().gap(6.0))
    )
    .style(|s| s.width_full().max_width_pct(90.0))
}

#[derive(Clone, Copy)]
enum TodoState {
    Completed,
    Pending,
}

// Single TODO item: <div class="flex flex-row items-start gap-2"><svg>...</svg><p>text</p></div>
fn todo_item_windsurf_dimmed(text: String, state: TodoState, is_dimmed: bool) -> impl View {
    let opacity = if is_dimmed { 0.5 } else { 1.0 };
    h_stack((
        // Icon: green checkmark or dashed circle
        match state {
            TodoState::Completed => {
                // Green checkmark - circle-check lucide icon
                label(|| "✓".to_string())
                    .style(move |s| {
                        s.width(16.0)
                            .height(16.0)
                            .margin_top(2.5)  // mt-[2.5px]
                            .flex_shrink(0.0)  // flex-shrink-0
                            .color(Color::from_rgb8(0x22, 0xc5, 0x5e).multiply_alpha(opacity))  // text-green-500
                            .font_size(14.0)
                            .font_weight(floem::text::Weight::BOLD)
                    })
            },
            TodoState::Pending => {
                // Dashed circle icon
                label(|| "○".to_string())
                    .style(move |s| {
                        s.width(16.0)
                            .height(16.0)
                            .margin_top(3.0)  // mt-[3px]
                            .flex_shrink(0.0)
                            .color(Color::from_rgb8(0xcc, 0xcc, 0xcc).multiply_alpha(opacity))
                            .font_size(14.0)
                    })
            },
        },
        
        // Text
        label(move || text.clone())
            .style(move |s| {
                s.font_size(13.0)
                    .line_height(1.5)
                    .color(Color::from_rgb8(0xcc, 0xcc, 0xcc).multiply_alpha(opacity))  // text-[var(--codeium-text-color)]
            }),
    ))
    .style(|s| {
        s.items_start()  // items-start
            .gap(8.0)  // gap-2 = 0.5rem = 8px
    })
}

fn todo_item_windsurf(text: String, state: TodoState) -> impl View {
    h_stack((
        // Icon: green checkmark or dashed circle
        match state {
            TodoState::Completed => {
                // Green checkmark - circle-check lucide icon
                label(|| "✓".to_string())
                    .style(|s| {
                        s.width(16.0)
                            .height(16.0)
                            .margin_top(2.5)  // mt-[2.5px]
                            .flex_shrink(0.0)  // flex-shrink-0
                            .color(Color::from_rgb8(0x22, 0xc5, 0x5e).multiply_alpha(0.5))  // text-green-500 at 50%
                            .font_size(14.0)
                            .font_weight(floem::text::Weight::BOLD)
                    })
            },
            TodoState::Pending => {
                // Dashed circle icon
                label(|| "○".to_string())
                    .style(|s| {
                        s.width(16.0)
                            .height(16.0)
                            .margin_top(3.0)  // mt-[3px]
                            .flex_shrink(0.0)
                            .color(Color::from_rgb8(0xcc, 0xcc, 0xcc).multiply_alpha(0.5))  // 50% opacity
                            .font_size(14.0)
                    })
            },
        },
        
        // Text
        label(move || text.clone())
            .style(|s| {
                s.font_size(13.0)
                    .line_height(1.5)
                    .color(Color::from_rgb8(0xcc, 0xcc, 0xcc).multiply_alpha(0.5))  // 50% opacity like Thought
            }),
    ))
    .style(|s| {
        s.items_start()  // items-start
            .gap(8.0)  // gap-2 = 0.5rem = 8px
    })
}

// Numbered TODO item (in-progress) - dimmed version
fn todo_item_windsurf_numbered_dimmed(num: usize, text: String, is_dimmed: bool) -> impl View {
    let opacity = if is_dimmed { 0.5 } else { 1.0 };
    h_stack((
        // Numbered circle: bg-[var(--codeium-text-color)] with number inside
        container(
            label(move || num.to_string())
                .style(move |s| {
                    s.font_size(8.0)  // text-[8px]
                        .font_weight(floem::text::Weight::BOLD)
                        .line_height(1.0)  // leading-none
                        .color(Color::from_rgb8(0x1f, 0x1f, 0x1f).multiply_alpha(opacity))  // text-[var(--vscode-editor-background)]
                })
        )
        .style(move |s| {
            s.width(16.0)  // w-[16px]
                .height(16.0)  // h-[16px]
                .margin_top(2.0)  // mt-[2px]
                .flex_shrink(0.0)
                .justify_center()
                .items_center()
                .border_radius(8.0)  // rounded-full (50%)
                .background(Color::from_rgb8(0xcc, 0xcc, 0xcc).multiply_alpha(opacity))  // bg-[var(--codeium-text-color)]
        }),
        
        // Text
        label(move || text.clone())
            .style(move |s| {
                s.font_size(13.0)
                    .line_height(1.5)
                    .color(Color::from_rgb8(0xcc, 0xcc, 0xcc).multiply_alpha(opacity))
            }),
    ))
    .style(|s| {
        s.items_start()
            .gap(8.0)
    })
}

// Numbered TODO item (in-progress)
fn todo_item_windsurf_numbered(num: usize, text: String) -> impl View {
    h_stack((
        // Numbered circle: bg-[var(--codeium-text-color)] with number inside
        container(
            label(move || num.to_string())
                .style(|s| {
                    s.font_size(8.0)  // text-[8px]
                        .font_weight(floem::text::Weight::BOLD)
                        .line_height(1.0)  // leading-none
                        .color(Color::from_rgb8(0x1f, 0x1f, 0x1f).multiply_alpha(0.5))  // 50% opacity
                })
        )
        .style(|s| {
            s.width(16.0)  // w-[16px]
                .height(16.0)  // h-[16px]
                .margin_top(2.0)  // mt-[2px]
                .flex_shrink(0.0)
                .justify_center()
                .items_center()
                .border_radius(8.0)  // rounded-full (50%)
                .background(Color::from_rgb8(0xcc, 0xcc, 0xcc).multiply_alpha(0.5))  // 50% opacity
        }),
        
        // Text
        label(move || text.clone())
            .style(|s| {
                s.font_size(13.0)
                    .line_height(1.5)
                    .color(Color::from_rgb8(0xcc, 0xcc, 0xcc).multiply_alpha(0.5))  // 50% opacity like Thought
            }),
    ))
    .style(|s| {
        s.items_start()
            .gap(8.0)  // gap-2
    })
}

// Status message WITHOUT feedback
fn ai_status_msg(items: Vec<&str>) -> impl View {
    ai_status_msg_internal(items, false)
}

// Status message WITH feedback (final)
fn ai_status_msg_final(items: Vec<&str>) -> impl View {
    ai_status_msg_internal(items, true)
}

// EXACT Windsurf Status with ai_msg structure
fn ai_status_msg_internal(items: Vec<&str>, show_feedback: bool) -> impl View {
    let status_items: Vec<String> = items.iter().map(|s| s.to_string()).collect();
    let status0 = status_items.clone();
    let status1 = status_items.clone();
    
    if show_feedback {
        container(
            v_stack((
                container(
                    h_stack((
                        h_stack((
                            label(|| "Thought".to_string()),
                            label(|| "for 1s".to_string())
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
                    v_stack((
                        label(|| "Status".to_string())
                            .style(|s| {
                                s.color(Color::from_rgb8(0xcc, 0xcc, 0xcc))
                                    .font_size(14.95)
                                    .font_weight(floem::text::Weight::SEMIBOLD)
                                    .margin_top(14.95)
                                    .margin_bottom(4.0)
                            }),
                        v_stack((
                            status_item_exact(status0.get(0).cloned().unwrap_or_default()),
                            status_item_exact(status1.get(1).cloned().unwrap_or_default()),
                        ))
                        .style(|s| s.flex_col().gap(4.0).margin(0.0)),
                    ))
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
                container(
                    h_stack((
                        h_stack((
                            label(|| "Thought".to_string()),
                            label(|| "for 1s".to_string())
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
                    v_stack((
                        label(|| "Status".to_string())
                            .style(|s| {
                                s.color(Color::from_rgb8(0xcc, 0xcc, 0xcc))
                                    .font_size(14.95)
                                    .font_weight(floem::text::Weight::SEMIBOLD)
                                    .margin_top(14.95)
                                    .margin_bottom(4.0)
                            }),
                        v_stack((
                            status_item_exact(status0.get(0).cloned().unwrap_or_default()),
                            status_item_exact(status1.get(1).cloned().unwrap_or_default()),
                        ))
                        .style(|s| s.flex_col().gap(4.0).margin(0.0)),
                    ))
                )
                .style(|s| s.width_full().gap(4.0)),
            ))
            .style(|s| s.flex_col().gap(6.0))
        )
        .style(|s| s.width_full().max_width_pct(90.0))
    }
}

// <li class="leading-[1.5] [&>p]:inline"><strong>Text</strong></li>
fn status_item_exact(text: String) -> impl View {
    h_stack((
        label(|| "•".to_string())  // Bullet point
            .style(|s| {
                s.font_size(13.0)
                    .line_height(1.5)
                    .color(Color::from_rgb8(0xcc, 0xcc, 0xcc))
                    .margin_right(6.0)
            }),
        
        label(move || text.clone())
            .style(|s| {
                s.font_size(13.0)
                    .line_height(1.5)
                    .font_weight(floem::text::Weight::SEMIBOLD)  // <strong>
                    .color(Color::from_rgb8(0xcc, 0xcc, 0xcc))
            }),
    ))
    .style(|s| {
        s.line_height(1.5)
    })
}

fn model_selector_dropdown() -> impl View {
    let is_open = create_rw_signal(false);
    let selected_model = create_rw_signal("Claude Sonnet 4.5 Thinking ".to_string());
    
    container(
        v_stack((
            // Dropdown panel (shown above when open)
            container(
                container(
                    v_stack((
                        // Search bar with filter
                        container(
                            h_stack((
                                label(|| "🔍".to_string())
                                    .style(|s| s.font_size(12.0)),
                                label(|| "Search all models".to_string())
                                    .style(|s| {
                                        s.font_size(12.0)
                                            .color(Color::from_rgb8(0xcc, 0xcc, 0xcc).multiply_alpha(0.5))
                                            .flex_grow(1.0)
                                    }),
                                label(|| "⚙".to_string())
                                    .style(|s| s.font_size(12.0)),
                                label(|| "?".to_string())
                                    .style(|s| s.font_size(12.0)),
                            ))
                            .style(|s| s.width_full().items_center().gap(4.0))
                        )
                        .style(|s| {
                            s.width_full()
                                .padding(4.0)
                                .background(Color::from_rgb8(0x80, 0x80, 0x80).multiply_alpha(0.1))
                                .border_radius(6.0)
                        }),
                        
                        // Recently Used section
                        v_stack((
                            label(|| "Recently Used".to_string())
                                .style(|s| {
                                    s.font_size(12.0)
                                        .color(Color::from_rgb8(0xcc, 0xcc, 0xcc).multiply_alpha(0.5))
                                        .padding_horiz(8.0)
                                        .padding_vert(4.0)
                                }),
                            // Claude Sonnet 4.5 Thinking (with checkmark)
                            container(
                                h_stack((
                                    label(|| "Claude Sonnet 4.5 Thinking ".to_string())
                                        .style(|s| s.font_size(12.0).color(Color::from_rgb8(0xcc, 0xcc, 0xcc)).flex_grow(1.0)),
                                    label(move || if selected_model.get() == "Claude Sonnet 4.5 Thinking " { "✓" } else { "" }.to_string())
                                        .style(|s| s.font_size(12.0).color(Color::from_rgb8(0xcc, 0xcc, 0xcc).multiply_alpha(0.5))),
                                ))
                                .style(|s| s.width_full())
                            )
                            .on_click_stop(move |_| {
                                selected_model.set("Claude Sonnet 4.5 Thinking ".to_string());
                                is_open.set(false);
                            })
                            .style(|s| {
                                s.width_full()
                                    .padding_horiz(8.0)
                                    .padding_vert(4.0)
                                    .border_radius(6.0)
                                    .cursor(floem::style::CursorStyle::Pointer)
                                    .hover(|s| s.background(Color::from_rgb8(0x80, 0x80, 0x80).multiply_alpha(0.1)))
                            }),
                            // Claude Sonnet 4
                            container(
                                label(|| "Claude Sonnet 4".to_string())
                                    .style(|s| s.font_size(12.0).color(Color::from_rgb8(0xcc, 0xcc, 0xcc)))
                            )
                            .on_click_stop(move |_| {
                                selected_model.set("Claude Sonnet 4".to_string());
                                is_open.set(false);
                            })
                            .style(|s| {
                                s.width_full()
                                    .padding_horiz(8.0)
                                    .padding_vert(4.0)
                                    .border_radius(6.0)
                                    .cursor(floem::style::CursorStyle::Pointer)
                                    .hover(|s| s.background(Color::from_rgb8(0x80, 0x80, 0x80).multiply_alpha(0.1)))
                            }),
                        ))
                        .style(|s| s.width_full().gap(2.0)),
                        
                        // Recommended section
                        v_stack((
                            label(|| "Recommended".to_string())
                                .style(|s| {
                                    s.font_size(12.0)
                                        .color(Color::from_rgb8(0xcc, 0xcc, 0xcc).multiply_alpha(0.5))
                                        .padding_horiz(8.0)
                                        .padding_vert(4.0)
                                }),
                            // Claude Sonnet 4.5 
                            container(
                                label(|| "Claude Sonnet 4.5 ".to_string())
                                    .style(|s| s.font_size(12.0).color(Color::from_rgb8(0xcc, 0xcc, 0xcc)))
                            )
                            .on_click_stop(move |_| {
                                selected_model.set("Claude Sonnet 4.5 ".to_string());
                                is_open.set(false);
                            })
                            .style(|s| {
                                s.width_full()
                                    .padding_horiz(8.0)
                                    .padding_vert(4.0)
                                    .border_radius(6.0)
                                    .cursor(floem::style::CursorStyle::Pointer)
                                    .hover(|s| s.background(Color::from_rgb8(0x80, 0x80, 0x80).multiply_alpha(0.1)))
                            }),
                            // GPT-5 (low reasoning)
                            container(
                                label(|| "GPT-5 (low reasoning)".to_string())
                                    .style(|s| s.font_size(12.0).color(Color::from_rgb8(0xcc, 0xcc, 0xcc)))
                            )
                            .on_click_stop(move |_| {
                                selected_model.set("GPT-5 (low reasoning)".to_string());
                                is_open.set(false);
                            })
                            .style(|s| {
                                s.width_full()
                                    .padding_horiz(8.0)
                                    .padding_vert(4.0)
                                    .border_radius(6.0)
                                    .cursor(floem::style::CursorStyle::Pointer)
                                    .hover(|s| s.background(Color::from_rgb8(0x80, 0x80, 0x80).multiply_alpha(0.1)))
                            }),
                            // Gemini 2.5 Pro
                            container(
                                label(|| "Gemini 2.5 Pro".to_string())
                                    .style(|s| s.font_size(12.0).color(Color::from_rgb8(0xcc, 0xcc, 0xcc)))
                            )
                            .on_click_stop(move |_| {
                                selected_model.set("Gemini 2.5 Pro".to_string());
                                is_open.set(false);
                            })
                            .style(|s| {
                                s.width_full()
                                    .padding_horiz(8.0)
                                    .padding_vert(4.0)
                                    .border_radius(6.0)
                                    .cursor(floem::style::CursorStyle::Pointer)
                                    .hover(|s| s.background(Color::from_rgb8(0x80, 0x80, 0x80).multiply_alpha(0.1)))
                            }),
                            // code-supernova-1-million
                            container(
                                label(|| "code-supernova-1-million".to_string())
                                    .style(|s| {
                                        s.font_size(12.0)
                                            .color(Color::from_rgb8(0xcc, 0xcc, 0xcc))
                                            .text_overflow(floem::style::TextOverflow::Wrap)
                                    })
                            )
                            .on_click_stop(move |_| {
                                selected_model.set("code-supernova-1-million".to_string());
                                is_open.set(false);
                            })
                            .style(|s| {
                                s.width_full()
                                    .padding_horiz(8.0)
                                    .padding_vert(4.0)
                                    .border_radius(6.0)
                                    .cursor(floem::style::CursorStyle::Pointer)
                                    .hover(|s| s.background(Color::from_rgb8(0x80, 0x80, 0x80).multiply_alpha(0.1)))
                            }),
                        ))
                        .style(|s| s.width_full().gap(2.0)),
                        
                        // See more button
                        label(|| "See more".to_string())
                            .style(|s| {
                                s.font_size(12.0)
                                    .color(Color::from_rgb8(0xcc, 0xcc, 0xcc).multiply_alpha(0.5))
                                    .padding_horiz(8.0)
                                    .padding_vert(4.0)
                                    .cursor(floem::style::CursorStyle::Pointer)
                                    .hover(|s| s.color(Color::from_rgb8(0xcc, 0xcc, 0xcc).multiply_alpha(0.8)))
                            }),
                    ))
                    .style(|s| s.flex_col().gap(4.0))
                )
                .style(|s| {
                    s.width(240.0)
                        .min_height(300.0)
                        .padding(6.0)
                        .padding_bottom(12.0)
                        .background(Color::from_rgb8(0x1a, 0x1a, 0x1a))
                        .border(1.0)
                        .border_color(Color::from_rgb8(0x45, 0x45, 0x45))
                        .border_radius(12.0)
                })
            )
            .style(move |s| {
                let mut style = if is_open.get() {
                    s.display(floem::style::Display::Flex)
                } else {
                    s.display(floem::style::Display::None)
                };
                style = style.position(floem::style::Position::Absolute)
                    .inset_bottom_pct(100.0)
                    .margin_bottom(4.0);
                style
            }),
            
            // Selector button
            container(
                h_stack((
                    label(move || selected_model.get())
                        .style(|s| {
                            s.font_size(12.0)
                                .color(Color::from_rgb8(0xcc, 0xcc, 0xcc))
                                .flex_grow(1.0)
                        }),
                    label(|| "▼".to_string())
                        .style(|s| {
                            s.font_size(10.0)
                                .color(Color::from_rgb8(0xcc, 0xcc, 0xcc).multiply_alpha(0.5))
                        }),
                ))
                .style(|s| s.width_full().items_center().gap(4.0))
            )
            .on_click_stop(move |_| {
                is_open.update(|v| *v = !*v);
            })
            .style(|s| {
                s.padding(2.0)
                    .padding_left(4.0)
                    .padding_right(4.0)
                    .border_radius(4.0)
                    .cursor(floem::style::CursorStyle::Pointer)
                    .hover(|s| s.background(Color::from_rgb8(0x80, 0x80, 0x80).multiply_alpha(0.1)))
            }),
        ))
        .style(|s| s.flex_col())
    )
    .style(|s| s.position(floem::style::Position::Relative))
}

fn input_bar() -> impl View {
    container(
        v_stack((
            // Text input area
            container(
                v_stack((
                    // Input field with placeholder
                    container(
                        text_input(create_rw_signal(String::new()))
                            .placeholder("Ask anything (Ctrl+L)".to_string())
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
                .style(|s| {
                    s.padding(2.0)
                        .padding_left(4.0)
                        .padding_right(4.0)
                        .border_radius(4.0)
                        .cursor(floem::style::CursorStyle::Pointer)
                        .hover(|s| s.background(Color::from_rgb8(0x80, 0x80, 0x80).multiply_alpha(0.1)))
                }),
                
                // Code button
                container(
                    label(|| "<> Code".to_string())
                        .style(|s| {
                            s.font_size(12.0)
                                .color(Color::from_rgb8(0xcc, 0xcc, 0xcc))
                        })
                )
                .style(|s| {
                    s.padding(2.0)
                        .padding_left(4.0)
                        .padding_right(4.0)
                        .border_radius(4.0)
                        .cursor(floem::style::CursorStyle::Pointer)
                        .hover(|s| s.background(Color::from_rgb8(0x80, 0x80, 0x80).multiply_alpha(0.1)))
                }),
                
                // Model selector with dropdown
                model_selector_dropdown(),
                
                // Spacer - pushes right buttons to the end
                container(label(|| "".to_string()))
                    .style(|s| s.flex_grow(1.0)),
                
                // Right buttons group (microphone + send)
                h_stack((
                    // Microphone button
                    container(
                        svg(|| r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 19v3"/><path d="M19 10v2a7 7 0 0 1-14 0v-2"/><rect x="9" y="2" width="6" height="13" rx="3"/></svg>"#.to_string())
                            .style(|s| {
                                s.width(14.0)
                                    .height(14.0)
                                    .color(Color::from_rgb8(0xcc, 0xcc, 0xcc))
                            })
                    )
                    .style(|s| {
                        s.padding(2.0)
                            .border_radius(4.0)
                            .cursor(floem::style::CursorStyle::Pointer)
                            .hover(|s| s.background(Color::from_rgb8(0x80, 0x80, 0x80).multiply_alpha(0.2)))
                    }),
                    
                    // Send button (disabled state)
                    container(
                        label(|| "↑".to_string())
                            .style(|s| {
                                s.color(Color::from_rgb8(0x1e, 0x1e, 0x1e))
                                    .font_size(12.0)
                                    .font_weight(floem::text::Weight::BOLD)
                            })
                    )
                    .style(|s| {
                        s.width(20.0)
                            .height(20.0)
                            .border_radius(10.0)
                            .background(Color::from_rgb8(0xcc, 0xcc, 0xcc).multiply_alpha(0.5))
                            .justify_center()
                            .items_center()
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
            .padding(6.0)
            .background(Color::from_rgb8(0x25, 0x25, 0x25))
            .border(1.0)
            .border_color(Color::from_rgb8(0x45, 0x45, 0x45))
            .border_radius(15.0)
            .margin(16.0)
    })
}
