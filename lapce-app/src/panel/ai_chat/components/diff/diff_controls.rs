// Diff Controls - Navigation and control panel for diffs
// Prev/next hunk, accept/reject all, view mode toggle

use std::sync::Arc;
use floem::{
    reactive::{RwSignal, SignalGet, SignalUpdate},
    views::{container, h_stack, label, v_stack, Decorators},
    View,
};
use crate::config::LapceConfig;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffViewMode {
    Unified,
    Split,
    Inline,
}

impl DiffViewMode {
    pub fn label(&self) -> &'static str {
        match self {
            DiffViewMode::Unified => "Unified",
            DiffViewMode::Split => "Split",
            DiffViewMode::Inline => "Inline",
        }
    }
}

#[derive(Debug, Clone)]
pub struct DiffControlsData {
    pub current_hunk: usize,
    pub total_hunks: usize,
    pub current_file: usize,
    pub total_files: usize,
}

/// Diff controls panel
/// Provides navigation and bulk actions for diff viewing
pub fn diff_controls(
    data: DiffControlsData,
    view_mode: RwSignal<DiffViewMode>,
    on_prev_hunk: impl Fn() + 'static + Copy,
    on_next_hunk: impl Fn() + 'static + Copy,
    on_prev_file: impl Fn() + 'static + Copy,
    on_next_file: impl Fn() + 'static + Copy,
    on_accept_all: impl Fn() + 'static + Copy,
    on_reject_all: impl Fn() + 'static + Copy,
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    v_stack((
        // File navigation
        h_stack((
            label(|| "File:".to_string())
                .style(move |s| {
                    let cfg = config();
                    s.color(cfg.color("editor.dim"))
                        .font_size(11.0)
                        .margin_right(8.0)
                }),
            
            container(
                label(|| "◀".to_string())
            )
            .on_click_stop(move |_| {
                if data.current_file > 1 {
                    on_prev_file();
                }
            })
            .style(move |s| {
                let cfg = config();
                let is_enabled = data.current_file > 1;
                s.padding(4.0)
                    .padding_horiz(8.0)
                    .border_radius(3.0)
                    .background(cfg.color("panel.current.background"))
                    .color(if is_enabled {
                        cfg.color("editor.foreground")
                    } else {
                        cfg.color("editor.dim")
                    })
                    .font_size(11.0)
                    .cursor(if is_enabled {
                        floem::style::CursorStyle::Pointer
                    } else {
                        floem::style::CursorStyle::Default
                    })
                    .margin_right(6.0)
            }),
            
            label({
                let current = data.current_file;
                let total = data.total_files;
                move || format!("{} / {}", current, total)
            })
            .style(move |s| {
                let cfg = config();
                s.color(cfg.color("editor.foreground"))
                    .font_family("monospace".to_string())
                    .font_size(11.0)
                    .margin_right(6.0)
            }),
            
            container(
                label(|| "▶".to_string())
            )
            .on_click_stop(move |_| {
                if data.current_file < data.total_files {
                    on_next_file();
                }
            })
            .style(move |s| {
                let cfg = config();
                let is_enabled = data.current_file < data.total_files;
                s.padding(4.0)
                    .padding_horiz(8.0)
                    .border_radius(3.0)
                    .background(cfg.color("panel.current.background"))
                    .color(if is_enabled {
                        cfg.color("editor.foreground")
                    } else {
                        cfg.color("editor.dim")
                    })
                    .font_size(11.0)
                    .cursor(if is_enabled {
                        floem::style::CursorStyle::Pointer
                    } else {
                        floem::style::CursorStyle::Default
                    })
            }),
        ))
        .style(|s| s.margin_bottom(12.0)),
        
        // Hunk navigation
        h_stack((
            label(|| "Hunk:".to_string())
                .style(move |s| {
                    let cfg = config();
                    s.color(cfg.color("editor.dim"))
                        .font_size(11.0)
                        .margin_right(8.0)
                }),
            
            container(
                label(|| "◀".to_string())
            )
            .on_click_stop(move |_| {
                if data.current_hunk > 1 {
                    on_prev_hunk();
                }
            })
            .style(move |s| {
                let cfg = config();
                let is_enabled = data.current_hunk > 1;
                s.padding(4.0)
                    .padding_horiz(8.0)
                    .border_radius(3.0)
                    .background(cfg.color("panel.current.background"))
                    .color(if is_enabled {
                        cfg.color("editor.foreground")
                    } else {
                        cfg.color("editor.dim")
                    })
                    .font_size(11.0)
                    .cursor(if is_enabled {
                        floem::style::CursorStyle::Pointer
                    } else {
                        floem::style::CursorStyle::Default
                    })
                    .margin_right(6.0)
            }),
            
            label({
                let current = data.current_hunk;
                let total = data.total_hunks;
                move || format!("{} / {}", current, total)
            })
            .style(move |s| {
                let cfg = config();
                s.color(cfg.color("editor.foreground"))
                    .font_family("monospace".to_string())
                    .font_size(11.0)
                    .margin_right(6.0)
            }),
            
            container(
                label(|| "▶".to_string())
            )
            .on_click_stop(move |_| {
                if data.current_hunk < data.total_hunks {
                    on_next_hunk();
                }
            })
            .style(move |s| {
                let cfg = config();
                let is_enabled = data.current_hunk < data.total_hunks;
                s.padding(4.0)
                    .padding_horiz(8.0)
                    .border_radius(3.0)
                    .background(cfg.color("panel.current.background"))
                    .color(if is_enabled {
                        cfg.color("editor.foreground")
                    } else {
                        cfg.color("editor.dim")
                    })
                    .font_size(11.0)
                    .cursor(if is_enabled {
                        floem::style::CursorStyle::Pointer
                    } else {
                        floem::style::CursorStyle::Default
                    })
            }),
        ))
        .style(|s| s.margin_bottom(16.0)),
        
        // View mode selector
        h_stack((
            label(|| "View:".to_string())
                .style(move |s| {
                    let cfg = config();
                    s.color(cfg.color("editor.dim"))
                        .font_size(11.0)
                        .margin_right(8.0)
                }),
            
            h_stack((
                container(
                    label(|| "Unified".to_string())
                )
                .on_click_stop(move |_| {
                    view_mode.set(DiffViewMode::Unified);
                })
                .style(move |s| {
                    let cfg = config();
                    let is_active = view_mode.get() == DiffViewMode::Unified;
                    s.padding(4.0)
                        .padding_horiz(10.0)
                        .border_radius(3.0)
                        .background(if is_active {
                            cfg.color("lapce.button.primary.background")
                        } else {
                            cfg.color("panel.current.background")
                        })
                        .color(if is_active {
                            cfg.color("lapce.button.primary.foreground")
                        } else {
                            cfg.color("editor.foreground")
                        })
                        .font_size(10.0)
                        .cursor(floem::style::CursorStyle::Pointer)
                        .margin_right(4.0)
                }),
                
                container(
                    label(|| "Split".to_string())
                )
                .on_click_stop(move |_| {
                    view_mode.set(DiffViewMode::Split);
                })
                .style(move |s| {
                    let cfg = config();
                    let is_active = view_mode.get() == DiffViewMode::Split;
                    s.padding(4.0)
                        .padding_horiz(10.0)
                        .border_radius(3.0)
                        .background(if is_active {
                            cfg.color("lapce.button.primary.background")
                        } else {
                            cfg.color("panel.current.background")
                        })
                        .color(if is_active {
                            cfg.color("lapce.button.primary.foreground")
                        } else {
                            cfg.color("editor.foreground")
                        })
                        .font_size(10.0)
                        .cursor(floem::style::CursorStyle::Pointer)
                        .margin_right(4.0)
                }),
                
                container(
                    label(|| "Inline".to_string())
                )
                .on_click_stop(move |_| {
                    view_mode.set(DiffViewMode::Inline);
                })
                .style(move |s| {
                    let cfg = config();
                    let is_active = view_mode.get() == DiffViewMode::Inline;
                    s.padding(4.0)
                        .padding_horiz(10.0)
                        .border_radius(3.0)
                        .background(if is_active {
                            cfg.color("lapce.button.primary.background")
                        } else {
                            cfg.color("panel.current.background")
                        })
                        .color(if is_active {
                            cfg.color("lapce.button.primary.foreground")
                        } else {
                            cfg.color("editor.foreground")
                        })
                        .font_size(10.0)
                        .cursor(floem::style::CursorStyle::Pointer)
                }),
            )),
        ))
        .style(|s| s.margin_bottom(16.0)),
        
        // Bulk actions
        h_stack((
            container(
                label(|| "Accept All".to_string())
            )
            .on_click_stop(move |_| {
                on_accept_all();
            })
            .style(move |s| {
                let cfg = config();
                s.padding(8.0)
                    .padding_horiz(16.0)
                    .border_radius(4.0)
                    .background(cfg.color("testing.iconPassed"))
                    .color(cfg.color("editor.background"))
                    .font_size(11.0)
                    .font_bold()
                    .cursor(floem::style::CursorStyle::Pointer)
                    .margin_right(8.0)
            }),
            
            container(
                label(|| "Reject All".to_string())
            )
            .on_click_stop(move |_| {
                on_reject_all();
            })
            .style(move |s| {
                let cfg = config();
                s.padding(8.0)
                    .padding_horiz(16.0)
                    .border_radius(4.0)
                    .background(cfg.color("list.errorForeground"))
                    .color(cfg.color("editor.background"))
                    .font_size(11.0)
                    .font_bold()
                    .cursor(floem::style::CursorStyle::Pointer)
            }),
        )),
    ))
    .style(move |s| {
        let cfg = config();
        s.padding(16.0)
            .border(1.0)
            .border_radius(6.0)
            .border_color(cfg.color("lapce.border"))
            .background(cfg.color("panel.background"))
    })
}
