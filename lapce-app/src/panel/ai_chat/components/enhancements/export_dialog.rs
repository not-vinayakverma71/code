// Export Dialog - Export conversation to various formats
// Save chat history as markdown, JSON, or plain text

use std::sync::Arc;
use floem::{
    reactive::{create_rw_signal, RwSignal, SignalGet, SignalUpdate},
    views::{container, h_stack, label, v_stack, Decorators},
    View,
};
use crate::config::LapceConfig;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    Markdown,
    Json,
    PlainText,
    Html,
}

impl ExportFormat {
    pub fn label(&self) -> &'static str {
        match self {
            ExportFormat::Markdown => "Markdown (.md)",
            ExportFormat::Json => "JSON (.json)",
            ExportFormat::PlainText => "Plain Text (.txt)",
            ExportFormat::Html => "HTML (.html)",
        }
    }
    
    pub fn description(&self) -> &'static str {
        match self {
            ExportFormat::Markdown => "Formatted text with code blocks",
            ExportFormat::Json => "Structured data format",
            ExportFormat::PlainText => "Simple text file",
            ExportFormat::Html => "Viewable in web browser",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ExportOptions {
    pub include_system_messages: bool,
    pub include_tool_calls: bool,
    pub include_timestamps: bool,
    pub include_metadata: bool,
}

impl Default for ExportOptions {
    fn default() -> Self {
        Self {
            include_system_messages: false,
            include_tool_calls: true,
            include_timestamps: true,
            include_metadata: false,
        }
    }
}

/// Format option helper
fn format_option(
    format: ExportFormat,
    selected_format: RwSignal<ExportFormat>,
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    let is_selected = selected_format.get() == format;
    container(
        h_stack((
            label(move || if is_selected { "●" } else { "○" })
                .style(|s| s.margin_right(8.0)),
            
            v_stack((
                label(move || format.label().to_string())
                    .style(move |s| {
                        let cfg = config();
                        s.color(cfg.color("editor.foreground"))
                            .font_size(12.0)
                    }),
                
                label(move || format.description().to_string())
                    .style(move |s| {
                        let cfg = config();
                        s.color(cfg.color("editor.dim"))
                            .font_size(10.0)
                    }),
            )),
        ))
    )
    .on_click_stop(move |_| {
        selected_format.set(format);
    })
    .style(move |s| {
        let cfg = config();
        s.padding(10.0)
            .width_full()
            .border(1.0)
            .border_radius(4.0)
            .border_color(if is_selected {
                cfg.color("lapce.button.primary.background")
            } else {
                cfg.color("input.border")
            })
            .background(if is_selected {
                cfg.color("list.activeSelectionBackground")
            } else {
                cfg.color("input.background")
            })
            .cursor(floem::style::CursorStyle::Pointer)
            .margin_bottom(8.0)
    })
}

/// Export dialog component
/// Configure and export conversation
pub fn export_dialog(
    on_export: impl Fn(ExportFormat, ExportOptions) + 'static + Copy,
    on_cancel: impl Fn() + 'static + Copy,
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    let selected_format = create_rw_signal(ExportFormat::Markdown);
    let options = create_rw_signal(ExportOptions::default());
    
    v_stack((
        // Header
        h_stack((
            label(|| "Export Conversation".to_string())
                .style(move |s| {
                    let cfg = config();
                    s.color(cfg.color("editor.foreground"))
                        .font_size(14.0)
                        .font_bold()
                        .flex_grow(1.0)
                }),
            
            container(
                label(|| "✕".to_string())
            )
            .on_click_stop(move |_| {
                on_cancel();
            })
            .style(move |s| {
                let cfg = config();
                s.padding(6.0)
                    .padding_horiz(10.0)
                    .border_radius(3.0)
                    .color(cfg.color("editor.foreground"))
                    .font_size(14.0)
                    .cursor(floem::style::CursorStyle::Pointer)
                    .hover(|s| s.background(cfg.color("list.hoverBackground")))
            }),
        ))
        .style(move |s| {
            let cfg = config();
            s.padding(16.0)
                .border_bottom(1.0)
                .border_color(cfg.color("lapce.border"))
                .background(cfg.color("titleBar.activeBackground"))
                .items_center()
        }),
        
        // Content
        v_stack((
            // Format selection (simplified static list)
            label(|| "Export Format:".to_string())
                .style(move |s| {
                    let cfg = config();
                    s.color(cfg.color("editor.foreground"))
                        .font_size(12.0)
                        .font_bold()
                        .margin_bottom(8.0)
                }),
            
            v_stack((
                // Markdown option
                format_option(ExportFormat::Markdown, selected_format, config),
                // JSON option
                format_option(ExportFormat::Json, selected_format, config),
                // Plain text option
                format_option(ExportFormat::PlainText, selected_format, config),
                // HTML option
                format_option(ExportFormat::Html, selected_format, config),
            ))
            .style(|s| s.margin_bottom(16.0)),
            
            // Options
            label(|| "Options:".to_string())
                .style(move |s| {
                    let cfg = config();
                    s.color(cfg.color("editor.foreground"))
                        .font_size(12.0)
                        .font_bold()
                        .margin_bottom(8.0)
                }),
            
            v_stack((
                // Include system messages
                h_stack((
                    container(
                        label(move || {
                            if options.get().include_system_messages { "☑" } else { "☐" }
                        })
                    )
                    .on_click_stop(move |_| {
                        options.update(|opts| opts.include_system_messages = !opts.include_system_messages);
                    })
                    .style(move |s| {
                        let cfg = config();
                        s.width(20.0)
                            .height(20.0)
                            .display(floem::style::Display::Flex)
                            .items_center()
                            .justify_center()
                            .border(1.0)
                            .border_radius(3.0)
                            .border_color(cfg.color("input.border"))
                            .cursor(floem::style::CursorStyle::Pointer)
                            .margin_right(8.0)
                    }),
                    
                    label(|| "Include system messages".to_string())
                        .style(move |s| {
                            let cfg = config();
                            s.color(cfg.color("editor.foreground"))
                                .font_size(12.0)
                        }),
                ))
                .style(|s| s.margin_bottom(8.0)),
                
                // Include tool calls
                h_stack((
                    container(
                        label(move || {
                            if options.get().include_tool_calls { "☑" } else { "☐" }
                        })
                    )
                    .on_click_stop(move |_| {
                        options.update(|opts| opts.include_tool_calls = !opts.include_tool_calls);
                    })
                    .style(move |s| {
                        let cfg = config();
                        s.width(20.0)
                            .height(20.0)
                            .display(floem::style::Display::Flex)
                            .items_center()
                            .justify_center()
                            .border(1.0)
                            .border_radius(3.0)
                            .border_color(cfg.color("input.border"))
                            .cursor(floem::style::CursorStyle::Pointer)
                            .margin_right(8.0)
                    }),
                    
                    label(|| "Include tool calls".to_string())
                        .style(move |s| {
                            let cfg = config();
                            s.color(cfg.color("editor.foreground"))
                                .font_size(12.0)
                        }),
                ))
                .style(|s| s.margin_bottom(8.0)),
                
                // Include timestamps
                h_stack((
                    container(
                        label(move || {
                            if options.get().include_timestamps { "☑" } else { "☐" }
                        })
                    )
                    .on_click_stop(move |_| {
                        options.update(|opts| opts.include_timestamps = !opts.include_timestamps);
                    })
                    .style(move |s| {
                        let cfg = config();
                        s.width(20.0)
                            .height(20.0)
                            .display(floem::style::Display::Flex)
                            .items_center()
                            .justify_center()
                            .border(1.0)
                            .border_radius(3.0)
                            .border_color(cfg.color("input.border"))
                            .cursor(floem::style::CursorStyle::Pointer)
                            .margin_right(8.0)
                    }),
                    
                    label(|| "Include timestamps".to_string())
                        .style(move |s| {
                            let cfg = config();
                            s.color(cfg.color("editor.foreground"))
                                .font_size(12.0)
                        }),
                ))
                .style(|s| s.margin_bottom(8.0)),
                
                // Include metadata
                h_stack((
                    container(
                        label(move || {
                            if options.get().include_metadata { "☑" } else { "☐" }
                        })
                    )
                    .on_click_stop(move |_| {
                        options.update(|opts| opts.include_metadata = !opts.include_metadata);
                    })
                    .style(move |s| {
                        let cfg = config();
                        s.width(20.0)
                            .height(20.0)
                            .display(floem::style::Display::Flex)
                            .items_center()
                            .justify_center()
                            .border(1.0)
                            .border_radius(3.0)
                            .border_color(cfg.color("input.border"))
                            .cursor(floem::style::CursorStyle::Pointer)
                            .margin_right(8.0)
                    }),
                    
                    label(|| "Include metadata (model, tokens, etc.)".to_string())
                        .style(move |s| {
                            let cfg = config();
                            s.color(cfg.color("editor.foreground"))
                                .font_size(12.0)
                        }),
                )),
            )),
        ))
        .style(|s| s.padding(16.0)),
        
        // Footer with actions
        h_stack((
            container(floem::views::empty())
                .style(|s| s.flex_grow(1.0)),
            
            container(
                label(|| "Cancel".to_string())
            )
            .on_click_stop(move |_| {
                on_cancel();
            })
            .style(move |s| {
                let cfg = config();
                s.padding(8.0)
                    .padding_horiz(16.0)
                    .border_radius(4.0)
                    .background(cfg.color("panel.current.background"))
                    .color(cfg.color("editor.foreground"))
                    .font_size(12.0)
                    .cursor(floem::style::CursorStyle::Pointer)
                    .margin_right(8.0)
            }),
            
            container(
                label(|| "Export".to_string())
            )
            .on_click_stop(move |_| {
                let format = selected_format.get();
                let opts = options.get();
                on_export(format, opts);
            })
            .style(move |s| {
                let cfg = config();
                s.padding(8.0)
                    .padding_horiz(16.0)
                    .border_radius(4.0)
                    .background(cfg.color("lapce.button.primary.background"))
                    .color(cfg.color("lapce.button.primary.foreground"))
                    .font_size(12.0)
                    .font_bold()
                    .cursor(floem::style::CursorStyle::Pointer)
            }),
        ))
        .style(move |s| {
            let cfg = config();
            s.padding(16.0)
                .border_top(1.0)
                .border_color(cfg.color("lapce.border"))
                .background(cfg.color("editor.background"))
        }),
    ))
    .style(move |s| {
        let cfg = config();
        s.width(500.0)
            .border(1.0)
            .border_radius(8.0)
            .border_color(cfg.color("lapce.border"))
            .background(cfg.color("editor.background"))
    })
}
