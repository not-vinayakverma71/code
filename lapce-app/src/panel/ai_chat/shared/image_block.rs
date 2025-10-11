// ImageBlock - ported from components/common/ImageBlock.tsx
// Phase 2: Basic image display support

use std::sync::Arc;

use floem::{
    reactive::SignalGet,
    views::{Decorators, container, empty, label, v_stack},
    IntoView, View,
};

use crate::config::LapceConfig;

pub struct ImageBlockProps {
    /// Webview-accessible URI for rendering the image
    pub image_uri: Option<String>,
    
    /// Actual file path for display purposes
    pub image_path: Option<String>,
    
    /// Base64 data or URL (backward compatibility)
    pub image_data: Option<String>,
}

/// Image block component
/// TODO Phase 3+: Add full image viewer with:
/// - Zoom controls
/// - Pan functionality  
/// - File path display
/// - Click to open in editor
pub fn image_block(
    props: ImageBlockProps,
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    let (final_uri, final_path) = if let Some(uri) = props.image_uri {
        (Some(uri), props.image_path)
    } else if let Some(data) = props.image_data {
        (Some(data), None)
    } else {
        (None, None)
    };
    
    if final_uri.is_none() {
        // No valid image data
        return container(
            label(|| "⚠ No image data provided".to_string())
                .style(move |s| {
                    let cfg = config();
                    s.padding(12.0)
                        .color(cfg.color("editor.errorForeground"))
                })
        )
        .style(|s| s.width_full())
        .into_any();
    }
    
    let uri = final_uri.unwrap();
    
    // Phase 2: Simple placeholder for images
    // TODO: Integrate actual image rendering when Floem supports it
    let path_label = if let Some(path) = final_path {
        label(move || format!("📷 {}", path.clone()))
            .style(move |s| {
                let cfg = config();
                s.padding(8.0)
                    .color(cfg.color("editor.dim"))
                    .font_size(11.0)
            })
            .into_any()
    } else {
        empty().into_any()
    };
    
    container(
        v_stack((
            path_label,
            
            // Image placeholder
            container(
                label(move || format!("🖼 Image: {}", 
                    if uri.starts_with("data:") { 
                        "base64 data" 
                    } else { 
                        &uri 
                    }
                ))
                .style(move |s| {
                    let cfg = config();
                    s.padding(20.0)
                        .color(cfg.color("editor.foreground"))
                })
            )
            .style(move |s| {
                let cfg = config();
                s.border(1.0)
                    .border_color(cfg.color("lapce.border"))
                    .border_radius(4.0)
                    .background(cfg.color("panel.background"))
                    .width_full()
            }),
        ))
    )
    .style(|s| s.padding(8.0).width_full())
    .into_any()
}
