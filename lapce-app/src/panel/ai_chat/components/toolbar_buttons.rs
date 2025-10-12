// Toolbar Buttons - History, File Upload, Image Upload
// Functional buttons with actual file/image pickers

use std::sync::Arc;

use floem::{
    action::open_file,
    file::FileDialogOptions,
    reactive::{RwSignal, SignalGet, SignalUpdate},
    views::{h_stack, label, Decorators},
    IntoView, View,
};

use crate::config::LapceConfig;

/// History button - toggles history panel
pub fn history_button(
    history_visible: RwSignal<bool>,
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    label(|| "📜".to_string()) // History icon
        .on_click_stop(move |_| {
            let current = history_visible.get();
            history_visible.set(!current);
        })
        .style(move |s| {
            let cfg = config();
            s.padding(8.0)
                .border_radius(4.0)
                .font_size(16.0)
                .cursor(floem::style::CursorStyle::Pointer)
                .hover(|s| {
                    s.background(cfg.color("panel.background"))
                })
        })
}

/// File upload button - opens actual file picker
pub fn file_upload_button(
    on_files_selected: impl Fn(Vec<String>) + 'static,
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    let on_files = Arc::new(on_files_selected);
    
    label(|| "📎".to_string()) // Paperclip/attachment icon
        .on_click_stop({
            let on_files = on_files.clone();
            move |_| {
                println!("[File Upload] Opening file picker...");
                
                // Open system file dialog with multi-selection
                let options = FileDialogOptions::new()
                    .title("Select Files to Attach")
                    .multi_selection();
                
                let on_files = on_files.clone();
                open_file(options, move |file_info| {
                    if let Some(file_info) = file_info {
                        let paths: Vec<String> = file_info.path
                            .iter()
                            .filter_map(|p| p.to_str().map(|s| s.to_string()))
                            .collect();
                        
                        println!("[File Upload] Selected {} files: {:?}", paths.len(), paths);
                        on_files(paths);
                    } else {
                        println!("[File Upload] Cancelled");
                    }
                });
            }
        })
        .style(move |s| {
            let cfg = config();
            s.padding(8.0)
                .border_radius(4.0)
                .font_size(16.0)
                .cursor(floem::style::CursorStyle::Pointer)
                .hover(|s| {
                    s.background(cfg.color("panel.background"))
                })
        })
}

/// Image upload button - opens actual image picker
pub fn image_upload_button(
    on_images_selected: impl Fn(Vec<String>) + 'static,
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    let on_images = Arc::new(on_images_selected);
    
    label(|| "🖼️".to_string()) // Image icon
        .on_click_stop({
            let on_images = on_images.clone();
            move |_| {
                println!("[Image Upload] Opening image picker...");
                
                // Open system file dialog filtered for images
                let mut options = FileDialogOptions::new()
                    .title("Select Images to Attach")
                    .multi_selection();
                
                // Add image file filters
                use floem::file::FileSpec;
                options = options.allowed_types(vec![
                    FileSpec {
                        name: "Images",
                        extensions: &["png", "jpg", "jpeg", "gif", "webp", "svg"],
                    },
                    FileSpec {
                        name: "All Files",
                        extensions: &["*"],
                    },
                ]);
                
                let on_images = on_images.clone();
                open_file(options, move |file_info| {
                    if let Some(file_info) = file_info {
                        let paths: Vec<String> = file_info.path
                            .iter()
                            .filter_map(|p| p.to_str().map(|s| s.to_string()))
                            .collect();
                        
                        println!("[Image Upload] Selected {} images: {:?}", paths.len(), paths);
                        on_images(paths);
                    } else {
                        println!("[Image Upload] Cancelled");
                    }
                });
            }
        })
        .style(move |s| {
            let cfg = config();
            s.padding(8.0)
                .border_radius(4.0)
                .font_size(16.0)
                .cursor(floem::style::CursorStyle::Pointer)
                .hover(|s| {
                    s.background(cfg.color("panel.background"))
                })
        })
}

/// Toolbar with all buttons
pub fn toolbar_buttons(
    history_visible: RwSignal<bool>,
    on_files_selected: impl Fn(Vec<String>) + 'static,
    on_images_selected: impl Fn(Vec<String>) + 'static,
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    h_stack((
        history_button(history_visible, config),
        file_upload_button(on_files_selected, config),
        image_upload_button(on_images_selected, config),
    ))
    .style(|s| s.gap(4.0).items_center())
}

/// Compact toolbar button with tooltip text (for future tooltip integration)
pub fn toolbar_button_with_label(
    icon: &'static str,
    label_text: &'static str,
    on_click: impl Fn() + 'static,
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    let on_click = Arc::new(on_click);
    
    label(move || icon.to_string())
        .on_click_stop({
            let on_click = on_click.clone();
            move |_| {
                on_click();
            }
        })
        .style(move |s| {
            let cfg = config();
            s.padding(8.0)
                .border_radius(4.0)
                .font_size(16.0)
                .cursor(floem::style::CursorStyle::Pointer)
                .hover(|s| {
                    s.background(cfg.color("panel.background"))
                })
        })
}
