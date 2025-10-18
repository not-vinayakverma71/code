// Test exact structure: signal passed to child function with nested v_stack
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
    let selected = create_rw_signal("Initial".to_string());
    
    v_stack((
        label(|| "Parent container".to_string()),
        
        // Child component (mimics input_bar calling model_selector_dropdown)
        dropdown_component(selected),
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

fn dropdown_component(selected: RwSignal<String>) -> impl View {
    let is_open = create_rw_signal(false);
    
    container(
        v_stack((
            // Dropdown panel (like our model list)
            container(
                v_stack((
                    container(label(|| "Option A".to_string()))
                        .on_click_stop(move |_| {
                            println!("[CLICK] Selected A");
                            selected.set("Option A".to_string());
                            is_open.set(false);
                        })
                        .style(|s| s.padding(8.0).background(Color::from_rgb8(0x40, 0x40, 0x40)).cursor(floem::style::CursorStyle::Pointer)),
                    
                    container(label(|| "Option B".to_string()))
                        .on_click_stop(move |_| {
                            println!("[CLICK] Selected B");
                            selected.set("Option B".to_string());
                            is_open.set(false);
                        })
                        .style(|s| s.padding(8.0).background(Color::from_rgb8(0x40, 0x40, 0x40)).cursor(floem::style::CursorStyle::Pointer)),
                ))
            )
            .style(move |s| {
                if is_open.get() {
                    s.padding(10.0).background(Color::from_rgb8(0x30, 0x30, 0x30))
                } else {
                    s.hide()
                }
            }),
            
            // Trigger button (like our model selector button)
            container(
                dyn_view(move || {
                    let val = selected.get();
                    println!("[RENDER] Button showing: {}", val);
                    label(move || val.clone())
                })
            )
            .on_click_stop(move |_| {
                println!("[CLICK] Toggling dropdown");
                is_open.update(|v| *v = !*v);
            })
            .style(|s| {
                s.padding(10.0)
                    .background(Color::from_rgb8(0x50, 0x50, 0x50))
                    .cursor(floem::style::CursorStyle::Pointer)
            }),
        ))
    )
}
