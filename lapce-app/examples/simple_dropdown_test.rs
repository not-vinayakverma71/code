// Minimal test to verify floem signal reactivity works
use floem::{
    peniko::Color,
    reactive::{create_effect, create_rw_signal, SignalGet, SignalUpdate},
    views::{container, dyn_view, h_stack, label, v_stack, Decorators},
    Application, View,
};

fn main() {
    Application::new()
        .window(|_| simple_test(), None)
        .run();
}

fn simple_test() -> impl View {
    let selected = create_rw_signal("Initial".to_string());
    
    create_effect(move |_| {
        println!("[DEBUG] selected changed to: {}", selected.get());
    });
    
    v_stack((
        // Display current value with dyn_view
        container(
            dyn_view(move || selected.get())
                .style(|s| {
                    s.font_size(20.0)
                        .color(Color::from_rgb8(0xff, 0xff, 0xff))
                        .padding(10.0)
                        .background(Color::from_rgb8(0x2a, 0x2a, 0x2a))
                        .border_radius(4.0)
                })
        ),
        
        // Buttons to change value
        h_stack((
            container(label(|| "Option A".to_string()))
                .on_click_stop(move |_| {
                    println!("[DEBUG] Clicked: Option A");
                    selected.set("Option A".to_string());
                })
                .style(|s| {
                    s.padding(8.0)
                        .background(Color::from_rgb8(0x40, 0x40, 0x40))
                        .border_radius(4.0)
                        .cursor(floem::style::CursorStyle::Pointer)
                        .hover(|s| s.background(Color::from_rgb8(0x60, 0x60, 0x60)))
                }),
            
            container(label(|| "Option B".to_string()))
                .on_click_stop(move |_| {
                    println!("[DEBUG] Clicked: Option B");
                    selected.set("Option B".to_string());
                })
                .style(|s| {
                    s.padding(8.0)
                        .background(Color::from_rgb8(0x40, 0x40, 0x40))
                        .border_radius(4.0)
                        .cursor(floem::style::CursorStyle::Pointer)
                        .hover(|s| s.background(Color::from_rgb8(0x60, 0x60, 0x60)))
                }),
            
            container(label(|| "Option C".to_string()))
                .on_click_stop(move |_| {
                    println!("[DEBUG] Clicked: Option C");
                    selected.set("Option C".to_string());
                })
                .style(|s| {
                    s.padding(8.0)
                        .background(Color::from_rgb8(0x40, 0x40, 0x40))
                        .border_radius(4.0)
                        .cursor(floem::style::CursorStyle::Pointer)
                        .hover(|s| s.background(Color::from_rgb8(0x60, 0x60, 0x60)))
                }),
        ))
        .style(|s| s.gap(10.0)),
    ))
    .style(|s| {
        s.flex_col()
            .gap(20.0)
            .padding(20.0)
            .width_full()
            .height_full()
            .items_center()
            .justify_center()
            .background(Color::from_rgb8(0x1a, 0x1a, 0x1a))
    })
}
