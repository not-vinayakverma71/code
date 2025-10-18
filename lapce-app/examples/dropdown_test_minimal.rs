// Minimal dropdown test - verify clicks work
use floem::{
    peniko::Color,
    reactive::{create_rw_signal, SignalGet, SignalUpdate},
    views::{container, h_stack, label, v_stack, Decorators},
    Application, View,
};

fn main() {
    Application::new()
        .window(|_| app_view(), None)
        .run();
}

fn app_view() -> impl View {
    let is_open = create_rw_signal(false);
    let selected = create_rw_signal("None".to_string());
    
    v_stack((
        // Simple dropdown items - ALWAYS VISIBLE for testing
        container(
            v_stack((
                label(|| "Item 1".to_string())
                    .on_click_stop(move |_| {
                        println!("[CLICK] Item 1");
                        selected.set("Item 1".to_string());
                    })
                    .style(|s| {
                        s.padding(8.0)
                            .cursor(floem::style::CursorStyle::Pointer)
                            .hover(|s| s.background(Color::from_rgb8(200, 200, 200)))
                    }),
                label(|| "Item 2".to_string())
                    .on_click_stop(move |_| {
                        println!("[CLICK] Item 2");
                        selected.set("Item 2".to_string());
                    })
                    .style(|s| {
                        s.padding(8.0)
                            .cursor(floem::style::CursorStyle::Pointer)
                            .hover(|s| s.background(Color::from_rgb8(200, 200, 200)))
                    }),
            ))
        )
        .style(|s| {
            s.width(200.0)
                .background(Color::from_rgb8(240, 240, 240))
                .border(1.0)
                .border_color(Color::BLACK)
        }),
        
        // Show selected
        label(move || format!("Selected: {}", selected.get()))
            .style(|s| s.margin_top(20.0)),
    ))
    .style(|s| {
        s.width_full()
            .height_full()
            .padding(50.0)
    })
}
