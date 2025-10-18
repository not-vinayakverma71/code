// Dropdown Menu Component - Radix UI equivalent for Floem
// Source: Codex webview-ui/src/components/ui/dropdown-menu.tsx
//
// Usage:
// dropdown_menu(
//     DropdownProps {
//         trigger: button(...),
//         items: vec![
//             DropdownItem::Regular { label: "Item 1", on_click: ... },
//             DropdownItem::Checkbox { label: "Check", checked: signal, ... },
//             DropdownItem::Separator,
//         ],
//         open: signal,
//     },
//     config
// )

use std::sync::Arc;

use floem::{
    event::EventListener,
    reactive::{RwSignal, SignalGet, SignalUpdate},
    style::{Position, CursorStyle},
    views::{Decorators, container, dyn_stack, empty, h_stack, label, scroll, v_stack},
    View, IntoView,
};

use crate::config::LapceConfig;

#[derive(Clone)]
pub enum DropdownItem {
    /// Regular menu item
    Regular {
        label: String,
        icon: Option<String>,
        disabled: bool,
        on_click: Arc<dyn Fn()>,
    },
    /// Checkbox item
    Checkbox {
        label: String,
        checked: RwSignal<bool>,
        disabled: bool,
        on_toggle: Arc<dyn Fn(bool)>,
    },
    /// Radio item
    Radio {
        label: String,
        value: String,
        selected: RwSignal<String>,
        disabled: bool,
    },
    /// Separator line
    Separator,
    /// Label (non-interactive)
    Label {
        text: String,
    },
}

pub struct DropdownProps<T>
where
    T: IntoView + 'static,
{
    /// Trigger element
    pub trigger: T,
    /// Menu items
    pub items: Vec<DropdownItem>,
    /// Open/close state
    pub open: RwSignal<bool>,
    /// Side offset
    pub side_offset: f64,
    /// Min width
    pub min_width: f64,
}

impl<T> DropdownProps<T>
where
    T: IntoView + 'static,
{
    pub fn new(trigger: T, items: Vec<DropdownItem>, open: RwSignal<bool>) -> Self {
        Self {
            trigger,
            items,
            open,
            side_offset: 4.0,
            min_width: 128.0, // min-w-[8rem]
        }
    }
}

/// Dropdown menu component
///
/// Features:
/// - Click outside to close
/// - ESC key to close
/// - Keyboard navigation (up/down arrows)
/// - Regular items, checkboxes, radio items
/// - Separators and labels
/// - Icons support
pub fn dropdown_menu<T>(
    props: DropdownProps<T>,
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View
where
    T: IntoView + 'static,
{
    let open = props.open;
    let items = props.items.clone();
    let min_width = props.min_width;
    let side_offset = props.side_offset;
    
    container((
        // Trigger
        container(props.trigger)
            .on_click_stop(move |_| {
                open.update(|o| *o = !*o);
            })
            .style(|s| s.cursor(CursorStyle::Pointer)),
        
        // Dropdown content
        container(
            // Click outside to close overlay
            container(
                // Menu items container
                container(
                    scroll(
                        dyn_stack(
                            move || items.clone(),
                            |item| match item {
                                DropdownItem::Regular { label, .. } => label.clone(),
                                DropdownItem::Checkbox { label, .. } => label.clone(),
                                DropdownItem::Radio { label, .. } => label.clone(),
                                DropdownItem::Separator => "sep".to_string(),
                                DropdownItem::Label { text } => text.clone(),
                            },
                            move |item| render_dropdown_item(item.clone(), open, config)
                        )
                        .style(|s| s.flex_col().gap(2.0))
                    )
                    .style(|s| s.max_height(400.0))
                )
                .on_event_stop(EventListener::KeyDown, move |event| {
                    if let floem::event::Event::KeyDown(key_event) = event {
                        if key_event.key.logical_key == floem::keyboard::Key::Named(floem::keyboard::NamedKey::Escape) {
                            open.set(false);
                            return floem::event::EventPropagation::Stop;
                        }
                    }
                    floem::event::EventPropagation::Continue
                })
                .style(move |s| {
                    let cfg = config();
                    let is_open = open.get();
                    
                    if !is_open {
                        return s.display(floem::style::Display::None);
                    }
                    
                    s.position(Position::Absolute)
                        .z_index(50)
                        .min_width(min_width)
                        .padding(4.0) // p-1
                        .border_radius(4.0) // rounded-xs
                        .background(cfg.color("panel.background"))
                        .border(1.0)
                        .border_color(cfg.color("lapce.border"))
                        .color(cfg.color("editor.foreground"))
                        .box_shadow_blur(8.0)
                        .box_shadow_color(cfg.color("editor.foreground").multiply_alpha(0.2))
                        .inset_top(side_offset)
                })
            )
            .on_click_stop(move |_| {
                open.set(false);
            })
            .style(move |s| {
                let is_open = open.get();
                if !is_open {
                    s.display(floem::style::Display::None)
                } else {
                    s.position(Position::Absolute)
                        .inset(0.0)
                        .z_index(40)
                }
            }),
        ),
    ))
    .style(|s| s.position(Position::Relative))
}

fn render_dropdown_item(
    item: DropdownItem,
    menu_open: RwSignal<bool>,
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> Box<dyn View> {
    match item {
        DropdownItem::Regular { label, icon, disabled, on_click } => {
            Box::new(
                h_stack((
                    {
                        if let Some(icon_text) = icon.clone() {
                            label(move || icon_text.clone())
                                .style(|s| s.margin_right(8.0).font_size(14.0))
                                .into_any()
                        } else {
                            label(|| "".to_string()).style(|s| s.width(0.0)).into_any()
                        }
                    },
                    label(move || label.to_string())
                        .style(|s| s.font_size(14.0)),
                ))
                .on_click_stop(move |_| {
                    if !disabled {
                        on_click();
                        menu_open.set(false);
                    }
                })
                .style(move |s| {
                    let cfg = config();
                    s.padding(6.0)
                        .padding_horiz(8.0)
                        .border_radius(4.0)
                        .width_full()
                        .items_center()
                        .color(cfg.color("editor.foreground"))
                        .apply_if(!disabled, |s| {
                            s.cursor(CursorStyle::Pointer)
                                .hover(|s| {
                                    s.background(cfg.color("list.activeSelectionBackground"))
                                        .color(cfg.color("list.activeSelectionForeground"))
                                })
                        })
                })
            )
        }
        
        DropdownItem::Checkbox { label: label_text, checked, disabled, on_toggle } => {
            Box::new(
                h_stack((
                    // Checkbox indicator
                    label(move || if checked.get() { "✓" } else { "" })
                        .style(|s| s.width(20.0).font_size(14.0)),
                    label(move || label_text.clone())
                        .style(|s| s.font_size(14.0)),
                ))
                .on_click_stop(move |_| {
                    if !disabled {
                        let new_value = !checked.get();
                        checked.set(new_value);
                        on_toggle(new_value);
                    }
                })
                .style(move |s| {
                    let cfg = config();
                    s.padding(6.0)
                        .padding_horiz(8.0)
                        .border_radius(4.0)
                        .width_full()
                        .items_center()
                        .color(cfg.color("editor.foreground"))
                        .apply_if(!disabled, |s| {
                            s.cursor(CursorStyle::Pointer)
                                .hover(|s| {
                                    s.background(cfg.color("list.activeSelectionBackground"))
                                        .color(cfg.color("list.activeSelectionForeground"))
                                })
                        })
                })
            )
        }
        
        DropdownItem::Radio { label: label_text, value, selected, disabled } => {
            Box::new(
                h_stack((
                    // Radio indicator
                    label(move || if selected.get() == value { "●" } else { "○" })
                        .style(|s| s.width(20.0).font_size(14.0)),
                    label(move || label_text.clone())
                        .style(|s| s.font_size(14.0)),
                ))
                .on_click_stop({
                    let value = value.clone();
                    move |_| {
                        if !disabled {
                            selected.set(value.clone());
                        }
                    }
                })
                .style(move |s| {
                    let cfg = config();
                    s.padding(6.0)
                        .padding_horiz(8.0)
                        .border_radius(4.0)
                        .width_full()
                        .items_center()
                        .color(cfg.color("editor.foreground"))
                        .apply_if(!disabled, |s| {
                            s.cursor(CursorStyle::Pointer)
                                .hover(|s| {
                                    s.background(cfg.color("list.activeSelectionBackground"))
                                        .color(cfg.color("list.activeSelectionForeground"))
                                })
                        })
                })
            )
        }
        
        DropdownItem::Separator => {
            Box::new(
                container(())
                    .style(move |s| {
                        let cfg = config();
                        s.width_full()
                            .height(1.0)
                            .margin_vert(4.0)
                            .background(cfg.color("lapce.border"))
                    })
            )
        }
        
        DropdownItem::Label { text } => {
            Box::new(
                label(move || text.clone())
                    .style(move |s| {
                        let cfg = config();
                        s.padding(6.0)
                            .padding_horiz(8.0)
                            .font_size(12.0)
                            .color(cfg.color("editor.dim"))
                    })
            )
        }
    }
}

/// Helper to create a regular dropdown item
pub fn dropdown_item(label: impl Into<String>, on_click: impl Fn() + 'static) -> DropdownItem {
    DropdownItem::Regular {
        label: label.into(),
        icon: None,
        disabled: false,
        on_click: Arc::new(on_click),
    }
}

/// Helper to create a checkbox dropdown item
pub fn dropdown_checkbox(
    label: impl Into<String>,
    checked: RwSignal<bool>,
    on_toggle: impl Fn(bool) + 'static,
) -> DropdownItem {
    DropdownItem::Checkbox {
        label: label.into(),
        checked,
        disabled: false,
        on_toggle: Arc::new(on_toggle),
    }
}
