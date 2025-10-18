// Test signal passing through function boundaries
use floem::{
    peniko::Color,
    reactive::{create_rw_signal, RwSignal, SignalGet, SignalUpdate},
    views::{container, dyn_view, h_stack, label, v_stack, Decorators},
    Application, View,
};

fn main() {
    Application::new()
        .window(|_| test_app(), None)
        .run();
}

fn test_app() -> impl View {
    // Signal created in parent
    let selected = create_rw_signal("Initial".to_string());
    
    v_stack((
        // Display in parent
        container(
            dyn_view(move || {
                label(move || format!("Parent sees: {}", selected.get()))
            })
        )
        .style(|s| s.padding(10.0).background(Color::from_rgb8(0x40, 0x40, 0x40))),
        
        // Pass signal to child function
        child_component(selected),
    ))
    .style(|s| {
        s.flex_col()
            .gap(20.0)
            .padding(20.0)
            .width_full()
            .height_full()
            .background(Color::from_rgb8(0x1a, 0x1a, 0x1a))
    })
}

// Child component receives signal
fn child_component(selected: RwSignal<String>) -> impl View {
    v_stack((
        // Display in child
        container(
            dyn_view(move || {
                label(move || format!("Child sees: {}", selected.get()))
            })
        )
        .style(|s| s.padding(10.0).background(Color::from_rgb8(0x30, 0x30, 0x30))),
        
        // Buttons in child
        h_stack((
            container(label(|| "Set A".to_string()))
                .on_click_stop(move |_| {
                    println!("[CLICK] Setting to A");
                    selected.set("Option A".to_string());
                })
                .style(|s| {
                    s.padding(8.0)
                        .background(Color::from_rgb8(0x50, 0x50, 0x50))
                        .cursor(floem::style::CursorStyle::Pointer)
                }),
            
            container(label(|| "Set B".to_string()))
                .on_click_stop(move |_| {
                    println!("[CLICK] Setting to B");
                    selected.set("Option B".to_string());
                })
                .style(|s| {
                    s.padding(8.0)
                        .background(Color::from_rgb8(0x50, 0x50, 0x50))
                        .cursor(floem::style::CursorStyle::Pointer)
                }),
        ))
        .style(|s| s.gap(10.0)),
    ))
    .style(|s| s.flex_col().gap(10.0))
}
