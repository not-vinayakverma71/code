// File Attachment System - Upload and preview files
// Matches Windsurf's file attachment UI

use std::sync::Arc;
use floem::{
    peniko::Color,
    reactive::{RwSignal, SignalGet, SignalUpdate},
    views::{Decorators, container, dyn_stack, h_stack, label, svg, v_stack},
    View,
};

use crate::{
    config::LapceConfig,
    panel::ai_chat::icons::*,
};

#[derive(Clone, Debug)]
pub struct AttachedFile {
    pub name: String,
    pub path: String,
    pub size: u64,
    pub file_type: FileType,
}

#[derive(Clone, Debug)]
pub enum FileType {
    Image,
    Code,
    Text,
    Binary,
}

pub struct FileAttachmentProps {
    pub attached_files: RwSignal<Vec<AttachedFile>>,
}

/// File attachment display and controls
pub fn file_attachment_list(
    props: FileAttachmentProps,
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    let attached_files = props.attached_files;
    
    container(
        container(
            dyn_stack(
                move || attached_files.get(),
                |file| format!("{}:{}", file.path, file.size),  // Unique key
                move |_file| {
                    let files = attached_files.get();
                    let idx = files.len().saturating_sub(1);  // Simple indexing
                    file_card(idx, attached_files, config)
                }
            )
        )
    )
    .style(move |s| {
        if attached_files.get().is_empty() {
            s.width(0.0).height(0.0)
        } else {
            s.width_full()
                .padding(8.0)
                .margin_bottom(8.0)
        }
    })
}

/// Individual file card
fn file_card(
    index: usize,
    attached_files: RwSignal<Vec<AttachedFile>>,
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    container(
        h_stack((
            // File icon
            file_icon(attached_files, index, config),
            
            // File info
            v_stack((
                // Filename
                label(move || {
                    attached_files.get()
                        .get(index)
                        .map(|f| f.name.clone())
                        .unwrap_or_default()
                })
                .style(move |s| {
                    let cfg = config();
                    s.font_size(13.0)
                        .color(cfg.color("editor.foreground"))
                }),
                
                // File size
                label(move || {
                    attached_files.get()
                        .get(index)
                        .map(|f| format_file_size(f.size))
                        .unwrap_or_default()
                })
                .style(move |s| {
                    let cfg = config();
                    s.font_size(11.0)
                        .color(cfg.color("editor.foreground").multiply_alpha(0.6))
                        .margin_top(2.0)
                }),
            ))
            .style(|s| s.flex_col().flex_grow(1.0)),
            
            // Remove button
            remove_button(index, attached_files, config),
        ))
        .style(|s| s.items_center().gap(8.0))
    )
    .style(move |s| {
        let cfg = config();
        s.width_full()
            .padding(8.0)
            .border_radius(6.0)
            .border(1.0)
            .border_color(cfg.color("lapce.border"))
            .background(cfg.color("editor.background").multiply_alpha(0.3))
    })
}

/// File type icon
fn file_icon(
    attached_files: RwSignal<Vec<AttachedFile>>,
    index: usize,
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    container(
        svg(move || {
            let file_type = attached_files.get()
                .get(index)
                .map(|f| f.file_type.clone())
                .unwrap_or(FileType::Text);
            
            match file_type {
                FileType::Image => ICON_PACKAGE,  // TODO: Better icon
                FileType::Code => ICON_CODE,
                FileType::Text => ICON_SEARCH,    // TODO: Better icon
                FileType::Binary => ICON_PACKAGE,
            }.to_string()
        })
        .style(move |s| {
            let cfg = config();
            s.width(16.0)
                .height(16.0)
                .color(cfg.color("editor.foreground").multiply_alpha(0.7))
        })
    )
    .style(move |s| {
        let cfg = config();
        s.width(32.0)
            .height(32.0)
            .border_radius(4.0)
            .justify_center()
            .items_center()
            .background(cfg.color("editor.foreground").multiply_alpha(0.1))
    })
}

/// Remove file button
fn remove_button(
    index: usize,
    attached_files: RwSignal<Vec<AttachedFile>>,
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    container(
        svg(|| ICON_X.to_string())
            .style(move |s| {
                let cfg = config();
                s.width(10.0)
                    .height(10.0)
                    .color(cfg.color("editor.foreground"))
            })
    )
    .on_click_stop(move |_| {
        attached_files.update(|files| {
            if index < files.len() {
                files.remove(index);
            }
        });
    })
    .style(move |s| {
        let cfg = config();
        s.padding(4.0)
            .border_radius(4.0)
            .cursor(floem::style::CursorStyle::Pointer)
            .color(cfg.color("editor.foreground").multiply_alpha(0.5))
            .hover(|s| {
                let cfg = config();
                s.background(cfg.color("editor.foreground").multiply_alpha(0.1))
                    .color(cfg.color("editor.foreground"))
            })
    })
}

/// Format file size
fn format_file_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.1} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}

/// File picker button (integrates with native file picker)
pub fn file_picker_button(
    on_files_selected: impl Fn(Vec<String>) + 'static + Copy,
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    container(
        svg(|| ICON_PLUS.to_string())
            .style(move |s| {
                let cfg = config();
                s.width(12.0)
                    .height(12.0)
                    .color(cfg.color("editor.foreground"))
            })
    )
    .on_click_stop(move |_| {
        // TODO: Open native file picker
        // For now, placeholder
        let files = vec![];
        on_files_selected(files);
    })
    .style(move |s| {
        let cfg = config();
        s.padding(4.0)
            .border_radius(4.0)
            .cursor(floem::style::CursorStyle::Pointer)
            .color(cfg.color("editor.foreground").multiply_alpha(0.7))
            .hover(|s| {
                let cfg = config();
                s.background(cfg.color("editor.foreground").multiply_alpha(0.1))
                    .color(cfg.color("editor.foreground"))
            })
    })
}
