// Test model selector in exact toolbar context like AI chat
use floem::{
    peniko::Color,
    reactive::{create_rw_signal, RwSignal, SignalGet, SignalUpdate},
    views::{container, dyn_view, h_stack, label, v_stack, text_input, Decorators},
    Application, View,
};

fn main() {
    Application::new()
        .window(|_| test_app(), None)
        .run();
}

fn test_app() -> impl View {
    let input_value = create_rw_signal(String::new());
    let selected_model = create_rw_signal("Model A".to_string());
    
    container(
        v_stack((
            // Text input
            container(
                text_input(input_value)
                    .placeholder("Type here".to_string())
            )
            .style(|s| s.width_full().padding(8.0).background(Color::from_rgb8(0x30, 0x30, 0x30))),
            
            // Bottom toolbar (EXACT structure from windsurf_ui.rs)
            h_stack((
                // Plus button
                container(label(|| "+".to_string()))
                    .on_click_stop(|_| println!("[+] Clicked"))
                    .style(|s| {
                        s.padding(8.0)
                            .cursor(floem::style::CursorStyle::Pointer)
                            .background(Color::from_rgb8(0x40, 0x40, 0x40))
                    }),
                
                // Model selector
                model_selector(selected_model),
                
                // Spacer
                container(label(|| "".to_string()))
                    .style(|s| s.flex_grow(1.0)),
                
                // Send button
                container(label(|| "↑".to_string()))
                    .on_click_stop(|_| println!("[Send] Clicked"))
                    .style(|s| {
                        s.padding(8.0)
                            .cursor(floem::style::CursorStyle::Pointer)
                            .background(Color::from_rgb8(0x40, 0x40, 0x40))
                    }),
            ))
            .style(|s| s.width_full().items_center().gap(8.0).padding(8.0)),
        ))
    )
    .style(|s| {
        s.width_full()
            .height_full()
            .background(Color::from_rgb8(0x1a, 0x1a, 0x1a))
    })
}

fn model_selector(selected: RwSignal<String>) -> impl View {
    let is_open = create_rw_signal(false);
    
    v_stack((
        // Dropdown
        container(
            v_stack((
                container(label(|| "Model A".to_string()))
                    .on_click_stop(move |_| {
                        println!("[Model] Selected A");
                        selected.set("Model A".to_string());
                        is_open.set(false);
                    })
                    .style(|s| s.padding(8.0).cursor(floem::style::CursorStyle::Pointer).background(Color::from_rgb8(0x40, 0x40, 0x40))),
                
                container(label(|| "Model B".to_string()))
                    .on_click_stop(move |_| {
                        println!("[Model] Selected B");
                        selected.set("Model B".to_string());
                        is_open.set(false);
                    })
                    .style(|s| s.padding(8.0).cursor(floem::style::CursorStyle::Pointer).background(Color::from_rgb8(0x40, 0x40, 0x40))),
            ))
        )
        .style(move |s| {
            if is_open.get() {
                s.position(floem::style::Position::Absolute)
                    .inset_bottom(100.0)
                    .inset_left(0.0)
                    .background(Color::from_rgb8(0x30, 0x30, 0x30))
                    .padding(10.0)
            } else {
                s.hide()
            }
        }),
        
        // Trigger button
        container(
            dyn_view(move || {
                let val = selected.get();
                println!("[Model Button] Showing: {}", val);
                label(move || format!("{} ▼", val.clone()))
            })
        )
        .on_click_stop(move |_| {
            println!("[Model Button] Clicked");
            is_open.update(|v| *v = !*v);
        })
        .style(|s| {
            s.padding(8.0)
                .cursor(floem::style::CursorStyle::Pointer)
                .background(Color::from_rgb8(0x50, 0x50, 0x50))
                .border_radius(4.0)
        }),
    ))
    .style(|s| s.position(floem::style::Position::Relative))
}
