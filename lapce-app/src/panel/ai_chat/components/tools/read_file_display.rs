// Read File Display - Windsurf simple style
// Just shows "Read filename.rs" with clickable filename

use std::sync::Arc;
use std::path::Path;
use floem::{
    peniko::Color,
    views::{container, h_stack, label, Decorators},
    View,
};
use crate::config::LapceConfig;

#[derive(Debug, Clone)]
pub struct ReadFileData {
    pub path: String,
}

/// Windsurf-style read file: "Read filename.rs" with clickable link
/// Example: Read <u>filename.rs</u>
pub fn read_file_display(
    data: ReadFileData,
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    // Extract just the filename from path
    let filename = Path::new(&data.path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(&data.path)
        .to_string();
    
    h_stack((
        // "Read" text in regular color
        label(|| "Read ".to_string())
            .style(move |s| {
                let cfg = config();
                s.color(cfg.color("editor.foreground"))
                    .font_size(13.0)
            }),
        
        // Clickable filename with underline on hover (Windsurf link style)
        container(
            label(move || filename.clone())
                .style(move |s| {
                    let cfg = config();
                    s.color(Color::from_rgb8(0x40, 0xa6, 0xff))  // #40a6ff blue link color
                        .font_size(13.0)
                        .font_family("monospace".to_string())
                })
        )
        .on_click_stop({
            let path = data.path.clone();
            move |_| {
                println!("[ReadFile] Open in editor: {}", path);
                // TODO: Wire to workspace.open_file
            }
        })
        .style(move |s| {
            s.cursor(floem::style::CursorStyle::Pointer)
                .hover(|s| {
                    // Underline on hover
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
