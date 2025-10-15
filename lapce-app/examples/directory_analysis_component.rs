// Directory Analysis Component - same styling as Read file indicator

use floem::{
    peniko::Color,
    views::{h_stack, label, Decorators},
    View,
};

fn directory_analysis_indicator(directory: &str) -> impl View {
    let dir_display = directory.to_string();
    let dir_click = directory.to_string();
    
    h_stack((
        label(|| "Analyzed".to_string())
            .style(|s| {
                s.font_size(13.0)
                    .color(Color::from_rgb8(0xcc, 0xcc, 0xcc).multiply_alpha(0.5))
            }),
        
        label(move || dir_display.clone())
            .on_click_stop(move |_| {
                println!("[Analyzed Directory] Open directory: {}", dir_click);
            })
            .style(|s| {
                s.font_size(13.0)
                    .color(Color::from_rgb8(0xcc, 0xcc, 0xcc).multiply_alpha(0.7))
                    .cursor(floem::style::CursorStyle::Pointer)
                    .hover(|s| s.color(Color::from_rgb8(0xcc, 0xcc, 0xcc)))
            }),
    ))
    .style(|s| s.gap(4.0).items_center().padding_vert(4.0))
}
