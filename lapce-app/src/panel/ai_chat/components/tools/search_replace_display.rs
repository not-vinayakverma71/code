// Search and Replace Display - Shows search/replace operations
// Pattern, replacement, files affected, matches count

use std::sync::Arc;
use floem::{
    reactive::{create_rw_signal, SignalGet},
    views::{container, h_stack, label, v_stack, Decorators},
    View,
};
use crate::{
    config::LapceConfig,
    panel::ai_chat::components::{
        messages::Status,
        tools::{tool_use_block, tool_metadata_row, ToolUseBlockProps},
    },
};

#[derive(Debug, Clone)]
pub struct SearchReplaceData {
    pub file_path: String,
    pub search_pattern: String,
    pub replace_pattern: String,
    pub matches_count: usize,
    pub replacements_made: usize,
    pub case_sensitive: bool,
    pub whole_word: bool,
    pub line_range: Option<(usize, usize)>,
    pub status: Status,
}

pub fn search_replace_display(
    data: SearchReplaceData,
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    let is_expanded = create_rw_signal(true);
    
    let content = v_stack((
        // File path
        tool_metadata_row(
            || "File:".to_string(),
            {
                let path = data.file_path.clone();
                move || path.clone()
            },
            config
        ),
        
        // Search pattern
        container(
            h_stack((
                label(|| "Search:".to_string())
                    .style(move |s| {
                        let cfg = config();
                        s.color(cfg.color("editor.dim"))
                            .font_size(11.0)
                            .margin_right(8.0)
                            .min_width(80.0)
                    }),
                
                container(
                    label({
                        let pattern = data.search_pattern.clone();
                        move || pattern.clone()
                    })
                )
                .style(move |s| {
                    let cfg = config();
                    s.padding(4.0)
                        .padding_horiz(8.0)
                        .border(1.0)
                        .border_radius(4.0)
                        .border_color(cfg.color("input.border"))
                        .background(cfg.color("input.background"))
                        .color(cfg.color("errorForeground"))
                        .font_family("monospace".to_string())
                        .font_size(11.0)
                }),
            ))
        )
        .style(|s| s.margin_bottom(6.0)),
        
        // Replace pattern
        container(
            h_stack((
                label(|| "Replace:".to_string())
                    .style(move |s| {
                        let cfg = config();
                        s.color(cfg.color("editor.dim"))
                            .font_size(11.0)
                            .margin_right(8.0)
                            .min_width(80.0)
                    }),
                
                container(
                    label({
                        let pattern = data.replace_pattern.clone();
                        move || pattern.clone()
                    })
                )
                .style(move |s| {
                    let cfg = config();
                    s.padding(4.0)
                        .padding_horiz(8.0)
                        .border(1.0)
                        .border_radius(4.0)
                        .border_color(cfg.color("input.border"))
                        .background(cfg.color("input.background"))
                        .color(cfg.color("testing.iconPassed"))
                        .font_family("monospace".to_string())
                        .font_size(11.0)
                }),
            ))
        )
        .style(|s| s.margin_bottom(6.0)),
        
        // Results
        tool_metadata_row(
            || "Matches:".to_string(),
            {
                let matches = data.matches_count;
                let replaced = data.replacements_made;
                move || format!("{} found, {} replaced", matches, replaced)
            },
            config
        ),
        
        // Options
        if data.case_sensitive || data.whole_word || data.line_range.is_some() {
            container(
                h_stack((
                    if data.case_sensitive {
                        container(
                            label(|| "Case Sensitive".to_string())
                        )
                        .style(move |s| {
                            let cfg = config();
                            s.padding(3.0)
                                .padding_horiz(6.0)
                                .border_radius(3.0)
                                .background(cfg.color("badge.background"))
                                .color(cfg.color("badge.foreground"))
                                .font_size(9.0)
                                .margin_right(6.0)
                        })
                    } else {
                        container(floem::views::empty())
                            .style(|s| s.display(floem::style::Display::None))
                    },
                    
                    if data.whole_word {
                        container(
                            label(|| "Whole Word".to_string())
                        )
                        .style(move |s| {
                            let cfg = config();
                            s.padding(3.0)
                                .padding_horiz(6.0)
                                .border_radius(3.0)
                                .background(cfg.color("badge.background"))
                                .color(cfg.color("badge.foreground"))
                                .font_size(9.0)
                                .margin_right(6.0)
                        })
                    } else {
                        container(floem::views::empty())
                            .style(|s| s.display(floem::style::Display::None))
                    },
                    
                    if let Some((start, end)) = data.line_range {
                        container(
                            label(move || format!("Lines {}-{}", start, end))
                        )
                        .style(move |s| {
                            let cfg = config();
                            s.padding(3.0)
                                .padding_horiz(6.0)
                                .border_radius(3.0)
                                .background(cfg.color("badge.background"))
                                .color(cfg.color("badge.foreground"))
                                .font_size(9.0)
                        })
                    } else {
                        container(floem::views::empty())
                            .style(|s| s.display(floem::style::Display::None))
                    },
                ))
            )
            .style(|s| s.margin_top(8.0))
        } else {
            container(floem::views::empty())
                .style(|s| s.display(floem::style::Display::None))
        },
        
        // Action button
        container(
            label(|| "Undo Changes".to_string())
        )
        .on_click_stop({
            let path = data.file_path.clone();
            move |_| {
                println!("[SearchReplace] Undo changes: {}", path);
                // TODO: Wire to undo mechanism when IPC ready
            }
        })
        .style(move |s| {
            let cfg = config();
            s.padding(6.0)
                .padding_horiz(12.0)
                .border_radius(4.0)
                .background(cfg.color("panel.current.background"))
                .color(cfg.color("editor.foreground"))
                .cursor(floem::style::CursorStyle::Pointer)
                .hover(|s| s.background(cfg.color("panel.hovered.background")))
                .margin_top(8.0)
        }),
    ));
    
    tool_use_block(
        ToolUseBlockProps {
            tool_name: "Search & Replace".to_string(),
            icon: "🔍".to_string(),
            status: data.status,
            is_expanded,
        },
        content,
        config
    )
}
