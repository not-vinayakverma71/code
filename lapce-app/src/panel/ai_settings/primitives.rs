// Floem UI Primitives for Settings Panels
// Reusable components: checkboxes, sliders, inputs, chips, etc.

use std::sync::Arc;
use floem::{
    reactive::{RwSignal, SignalGet, SignalUpdate},
    views::{
        container, dyn_stack, empty, h_stack, label, stack, v_stack,
        text_input, Decorators,
    },
    IntoView, View,
};
use crate::config::LapceConfig;

// ============================================================
// Item 79: Vertical tabs scaffold
// ============================================================

pub struct VerticalTabsProps {
    pub tabs: Vec<(&'static str, Box<dyn Fn() -> Box<dyn View>>)>,
    pub active_tab: RwSignal<usize>,
}

pub fn vertical_tabs(props: VerticalTabsProps, config: impl Fn() -> Arc<LapceConfig> + 'static + Copy) -> impl View {
    let VerticalTabsProps { tabs, active_tab } = props;
    
    // Simplified vertical tabs - just use placeholder for now
    empty().style(|s| s.width_full().height_full())
}

// ============================================================
// Item 80: Section header component
// ============================================================

pub fn section_header(title: impl Fn() -> String + 'static, config: impl Fn() -> Arc<LapceConfig> + 'static + Copy) -> impl View {
    label(title)
        .style(move |s| {
            let cfg = config();
            s.font_size(14.0)
                .font_bold()
                .color(cfg.color("editor.foreground"))
                .margin_bottom(12.0)
        })
}

// ============================================================
// Item 81: Checkbox row with description
// ============================================================

pub fn checkbox_row(
    label_text: impl Fn() -> String + 'static,
    description: impl Fn() -> String + 'static,
    checked: RwSignal<bool>,
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    v_stack((
        h_stack((
            // Checkbox visual (simplified - real checkbox would use native widget)
            container(empty())
                .on_click_stop(move |_| {
                    checked.update(|v| *v = !*v);
                })
                .style(move |s| {
                    let cfg = config();
                    let base = s
                        .width(16.0)
                        .height(16.0)
                        .border(1.0)
                        .border_radius(3.0)
                        .border_color(cfg.color("input.border"))
                        .cursor(floem::style::CursorStyle::Pointer);
                    
                    if checked.get() {
                        base.background(cfg.color("button.background"))
                    } else {
                        base
                    }
                }),
            
            label(label_text)
                .style(move |s| {
                    let cfg = config();
                    s.margin_left(8.0)
                        .color(cfg.color("editor.foreground"))
                        .cursor(floem::style::CursorStyle::Pointer)
                })
                .on_click_stop(move |_| {
                    checked.update(|v| *v = !*v);
                }),
        ))
        .style(|s| s.items_center()),
        
        label(description)
            .style(move |s| {
                let cfg = config();
                s.margin_top(4.0)
                    .margin_left(24.0)
                    .font_size(11.0)
                    .color(cfg.color("descriptionForeground"))
            }),
    ))
    .style(|s| s.margin_bottom(16.0))
}

// ============================================================
// Item 82: Slider component with value label
// ============================================================

pub fn slider_row(
    label_text: impl Fn() -> String + 'static,
    value: RwSignal<f32>,
    min: f32,
    max: f32,
    step: f32,
    format_fn: impl Fn(f32) -> String + 'static,
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    v_stack((
        label(label_text)
            .style(move |s| {
                let cfg = config();
                s.color(cfg.color("editor.foreground"))
                    .margin_bottom(8.0)
            }),
        
        h_stack((
            // Slider track (simplified)
            container(empty())
                .style(move |s| {
                    let cfg = config();
                    s.width(300.0)
                        .height(4.0)
                        .background(cfg.color("input.background"))
                        .border_radius(2.0)
                }),
            
            // Value display
            label(move || format_fn(value.get()))
                .style(move |s| {
                    let cfg = config();
                    s.margin_left(12.0)
                        .min_width(60.0)
                        .color(cfg.color("editor.foreground"))
                }),
        ))
        .style(|s| s.items_center()),
    ))
    .style(|s| s.margin_bottom(16.0))
}

// ============================================================
// Item 83: Key-value JSON editor placeholder
// ============================================================

pub fn json_editor(
    label_text: impl Fn() -> String + 'static,
    value: RwSignal<String>,
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    v_stack((
        label(label_text)
            .style(move |s| {
                let cfg = config();
                s.color(cfg.color("editor.foreground"))
                    .margin_bottom(8.0)
            }),
        
        text_input(value)
            .style(move |s| {
                let cfg = config();
                s.width_full()
                    .padding(8.0)
                    .border(1.0)
                    .border_radius(4.0)
                    .border_color(cfg.color("input.border"))
                    .background(cfg.color("input.background"))
                    .color(cfg.color("input.foreground"))
            }),
    ))
    .style(|s| s.margin_bottom(16.0))
}

// ============================================================
// Item 84: Removable chip list (for command lists)
// ============================================================

pub fn chip_list(
    items: RwSignal<Vec<String>>,
    on_remove: impl Fn(usize) + 'static + Copy,
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    // Simplified chip list - placeholder for now
    label(move || format!("{} items", items.get().len()))
        .style(move |s| {
            let cfg = config();
            s.color(cfg.color("editor.foreground"))
        })
}

// ============================================================
// Item 85-94: Additional primitives (stubs for now)
// ============================================================

// Item 85: Multi-select dropdown (uses Floem Dropdown)
// Item 86: Text input with units
pub fn text_input_with_units(
    value: RwSignal<String>,
    units: &'static str,
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    h_stack((
        text_input(value)
            .style(move |s| {
                let cfg = config();
                s.width(200.0)
                    .padding(6.0)
                    .border(1.0)
                    .border_radius(4.0)
                    .border_color(cfg.color("input.border"))
                    .background(cfg.color("input.background"))
            }),
        
        label(move || units.to_string())
            .style(move |s| {
                let cfg = config();
                s.margin_left(8.0)
                    .color(cfg.color("descriptionForeground"))
            }),
    ))
    .style(|s| s.items_center())
}

// Item 87: Tooltip wrapper (Floem has built-in tooltip)
// Item 88: Info banner
pub fn info_banner(message: impl Fn() -> String + 'static, config: impl Fn() -> Arc<LapceConfig> + 'static + Copy) -> impl View {
    container(
        label(message)
    )
    .style(move |s| {
        let cfg = config();
        s.padding(12.0)
            .border_radius(4.0)
            .background(cfg.color("editorInfo.background"))
            .color(cfg.color("editorInfo.foreground"))
            .margin_bottom(16.0)
    })
}

// Item 89: Form footer (Save/Done buttons)
pub fn form_footer(
    on_save: impl Fn() + 'static + Copy,
    on_cancel: impl Fn() + 'static + Copy,
    save_enabled: impl Fn() -> bool + 'static + Copy,
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    h_stack((
        container(
            label(|| "Save".to_string())
        )
        .on_click_stop(move |_| {
            if save_enabled() {
                on_save();
            }
        })
        .style(move |s| {
            let cfg = config();
            let base = s
                .padding(8.0)
                .padding_horiz(16.0)
                .border_radius(4.0)
                .cursor(floem::style::CursorStyle::Pointer);
            
            if save_enabled() {
                base.background(cfg.color("button.background"))
                    .color(cfg.color("button.foreground"))
            } else {
                base.background(cfg.color("button.secondaryBackground"))
                    .color(cfg.color("disabledForeground"))
            }
        }),
        
        container(
            label(|| "Cancel".to_string())
        )
        .on_click_stop(move |_| {
            on_cancel();
        })
        .style(move |s| {
            let cfg = config();
            s.margin_left(8.0)
                .padding(8.0)
                .padding_horiz(16.0)
                .border_radius(4.0)
                .background(cfg.color("button.secondaryBackground"))
                .color(cfg.color("button.secondaryForeground"))
                .cursor(floem::style::CursorStyle::Pointer)
        }),
    ))
}

// Items 90-94: Additional primitives (list with pin/unpin, table, collapsible, search, validation)
// These are stubs - to be implemented as needed for specific panels

pub fn collapsible_group(
    title: impl Fn() -> String + 'static,
    expanded: RwSignal<bool>,
    content: impl View + 'static,
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    v_stack((
        h_stack((
            label(move || if expanded.get() { "▼" } else { "▶" })
                .style(|s| s.margin_right(8.0)),
            label(title),
        ))
        .on_click_stop(move |_| {
            expanded.update(|v| *v = !*v);
        })
        .style(move |s| {
            let cfg = config();
            s.padding(8.0)
                .cursor(floem::style::CursorStyle::Pointer)
                .color(cfg.color("editor.foreground"))
        }),
        
        container(content)
            .style(move |s| {
                if expanded.get() {
                    s.padding_left(20.0)
                } else {
                    s.display(floem::style::Display::None)
                }
            }),
    ))
}
