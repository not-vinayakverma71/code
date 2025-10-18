// File edit component function for reference

fn file_edit_item(filename: &str, additions: i32, deletions: i32) -> impl View {
    let fname = filename.to_string();
    let fname_click = filename.to_string();
    
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
                            .color(Color::from_rgb8(0x6a, 0x99, 0x56))  // green
                    }),
                label(move || format!("-{}", deletions))
                    .style(|s| {
                        s.font_size(13.0)
                            .font_weight(floem::text::Weight::MEDIUM)
                            .color(Color::from_rgb8(0xff, 0x00, 0x00).multiply_alpha(0.7))  // red
                    }),
            ))
            .style(|s| s.gap(4.0).flex_shrink(0.0).margin_left(8.0)),
        ))
    )
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
                s.background(Color::from_rgb8(0x73, 0x73, 0x73).multiply_alpha(0.1))  // hover:bg-neutral-500/10
            })
    })
}
