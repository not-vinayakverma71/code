// Error while editing component function for reference

fn ai_error_editing_file(filename: &str) -> impl View {
    let fname = filename.to_string();
    let fname_click = filename.to_string();
    
    container(
        h_stack((
            // Left side: "Error while editing" + filename
            h_stack((
                label(|| "Error while editing".to_string())
                    .style(|s| {
                        s.font_size(13.0)
                            .color(Color::from_rgb8(0xff, 0x63, 0x47))  // Red for error
                    }),
                
                // Filename (clickable with file icon)
                label(move || fname.clone())
                    .on_click_stop(move |_| {
                        println!("[Error File] Open: {}", fname_click);
                    })
                    .style(|s| {
                        s.font_size(13.0)
                            .font_family("monospace".to_string())
                            .color(Color::from_rgb8(0x9c, 0xda, 0xfe))  // Link color
                            .cursor(floem::style::CursorStyle::Pointer)
                            .hover(|s| {
                                s.color(Color::from_rgb8(0xb0, 0xe5, 0xff))  // Brighter on hover
                            })
                    }),
            ))
            .style(|s| s.gap(6.0).items_center()),
            
            // Right side: chevron (hidden by default, shown on hover)
            label(|| "›".to_string())
                .style(|s| {
                    s.font_size(12.0)
                        .color(Color::from_rgb8(0xcc, 0xcc, 0xcc).multiply_alpha(0.0))
                }),
        ))
        .style(|s| s.items_center().gap(4.0))
    )
    .style(|s| {
        s.width_full()
            .max_width_pct(90.0)
            .padding_vert(4.0)
            .cursor(floem::style::CursorStyle::Pointer)
    })
}
