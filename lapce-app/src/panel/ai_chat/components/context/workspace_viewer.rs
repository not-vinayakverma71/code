// Workspace Viewer - Browse and select workspace files
// Tree view of project structure with file selection

use std::sync::Arc;
use floem::{
    reactive::{create_rw_signal, SignalGet, SignalUpdate},
    views::{container, h_stack, label, scroll, v_stack, Decorators},
    View,
};
use crate::config::LapceConfig;

#[derive(Debug, Clone)]
pub struct WorkspaceFile {
    pub path: String,
    pub name: String,
    pub is_dir: bool,
    pub is_expanded: bool,
    pub depth: usize,
    pub size_bytes: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct WorkspaceViewerData {
    pub workspace_root: String,
    pub files: Vec<WorkspaceFile>,
    pub total_files: usize,
    pub total_size_mb: f64,
}

/// Workspace viewer component
/// Browse project files in tree structure
pub fn workspace_viewer(
    data: WorkspaceViewerData,
    on_select_file: impl Fn(String) + 'static + Copy,
    on_toggle_dir: impl Fn(String) + 'static + Copy,
    on_refresh: impl Fn() + 'static + Copy,
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    let search_query = create_rw_signal(String::new());
    let is_expanded = create_rw_signal(true);
    
    v_stack((
        // Header
        h_stack((
            label(move || if is_expanded.get() { "▼" } else { "▶" })
                .on_click_stop(move |_| {
                    is_expanded.update(|v| *v = !*v);
                })
                .style(move |s| {
                    let cfg = config();
                    s.color(cfg.color("editor.foreground"))
                        .font_size(12.0)
                        .margin_right(8.0)
                        .cursor(floem::style::CursorStyle::Pointer)
                }),
            
            label(|| "Workspace".to_string())
                .style(move |s| {
                    let cfg = config();
                    s.color(cfg.color("editor.foreground"))
                        .font_size(13.0)
                        .font_bold()
                        .flex_grow(1.0)
                }),
            
            // Stats
            label({
                let files = data.total_files;
                let size = data.total_size_mb;
                move || format!("{} files • {:.1} MB", files, size)
            })
            .style(move |s| {
                let cfg = config();
                s.color(cfg.color("editor.dim"))
                    .font_size(10.0)
                    .margin_right(12.0)
            }),
            
            // Refresh button
            container(
                label(|| "🔄".to_string())
            )
            .on_click_stop(move |_| {
                on_refresh();
            })
            .style(move |s| {
                let cfg = config();
                s.padding(4.0)
                    .padding_horiz(8.0)
                    .border_radius(3.0)
                    .background(cfg.color("panel.current.background"))
                    .color(cfg.color("editor.foreground"))
                    .font_size(11.0)
                    .cursor(floem::style::CursorStyle::Pointer)
                    .hover(|s| s.background(cfg.color("panel.hovered.background")))
            }),
        ))
        .style(move |s| {
            let cfg = config();
            s.padding(12.0)
                .border_bottom(1.0)
                .border_color(cfg.color("lapce.border"))
                .background(cfg.color("panel.background"))
                .items_center()
        }),
        
        // Content
        container(
            v_stack((
                // Workspace root
                label({
                    let root = data.workspace_root.clone();
                    move || format!("📁 {}", root)
                })
                .style(move |s| {
                    let cfg = config();
                    s.color(cfg.color("editor.foreground"))
                        .font_family("monospace".to_string())
                        .font_size(11.0)
                        .font_bold()
                        .margin_bottom(12.0)
                }),
                
                // File tree (simplified text view)
                scroll(
                    label({
                        let files = data.files.clone();
                        let query = search_query;
                        move || {
                            let q = query.get();
                            files.iter()
                                .filter(|f| {
                                    if q.is_empty() {
                                        true
                                    } else {
                                        f.name.to_lowercase().contains(&q.to_lowercase())
                                    }
                                })
                                .map(|file| {
                                    let indent = "  ".repeat(file.depth);
                                    let icon = if file.is_dir {
                                        if file.is_expanded { "📂" } else { "📁" }
                                    } else {
                                        "📄"
                                    };
                                    
                                    let size_info = if let Some(bytes) = file.size_bytes {
                                        if bytes >= 1024 * 1024 {
                                            format!(" ({:.1} MB)", bytes as f64 / (1024.0 * 1024.0))
                                        } else if bytes >= 1024 {
                                            format!(" ({:.1} KB)", bytes as f64 / 1024.0)
                                        } else {
                                            format!(" ({} B)", bytes)
                                        }
                                    } else {
                                        String::new()
                                    };
                                    
                                    format!("{}{} {}{}", indent, icon, file.name, size_info)
                                })
                                .collect::<Vec<_>>()
                                .join("\n")
                        }
                    })
                    .style(move |s| {
                        let cfg = config();
                        s.color(cfg.color("editor.foreground"))
                            .font_family("monospace".to_string())
                            .font_size(11.0)
                            .line_height(1.6)
                    })
                )
                .style(|s| s.max_height(400.0).width_full()),
                
                // Helper text
                label(|| "💡 Click files to attach to context".to_string())
                    .style(move |s| {
                        let cfg = config();
                        s.color(cfg.color("editor.dim"))
                            .font_size(10.0)
                            .font_style(floem::text::Style::Italic)
                            .margin_top(12.0)
                    }),
            ))
            .style(|s| s.padding(12.0))
        )
        .style(move |s| {
            if is_expanded.get() {
                s
            } else {
                s.display(floem::style::Display::None)
            }
        }),
    ))
    .style(move |s| {
        let cfg = config();
        s.width_full()
            .border(1.0)
            .border_radius(6.0)
            .border_color(cfg.color("lapce.border"))
            .background(cfg.color("editor.background"))
            .margin_bottom(16.0)
    })
}
