// Popover Component - Radix UI equivalent for Floem
// Source: Codex webview-ui/src/components/ui/popover.tsx
//
// Usage:
// popover(
//     PopoverProps {
//         trigger: button(...),
//         content: v_stack((...)),
//         open: signal,
//         position: PopoverPosition::Bottom,
//     },
//     config
// )

use std::sync::Arc;

use floem::{
    event::{EventListener, EventPropagation},
    keyboard::Key,
    reactive::{RwSignal, SignalGet, SignalUpdate},
    style::{CursorStyle, Position},
    views::{Decorators, container, stack},
    View, IntoView,
};

use crate::config::LapceConfig;

#[derive(Clone, Copy, PartialEq)]
pub enum PopoverPosition {
    Top,
    Bottom,
    Left,
    Right,
    TopStart,
    TopEnd,
    BottomStart,
    BottomEnd,
}

pub struct PopoverProps<T, C>
where
    T: IntoView + 'static,
    C: IntoView + 'static,
{
    /// Trigger element (button, etc.)
    pub trigger: T,
    /// Content to show in popover
    pub content: C,
    /// Open/close state
    pub open: RwSignal<bool>,
    /// Position relative to trigger
    pub position: PopoverPosition,
    /// Side offset in pixels
    pub side_offset: f64,
}

impl<T, C> PopoverProps<T, C>
where
    T: IntoView + 'static,
    C: IntoView + 'static,
{
    pub fn new(trigger: T, content: C, open: RwSignal<bool>) -> Self {
        Self {
            trigger,
            content,
            open,
            position: PopoverPosition::Bottom,
            side_offset: 4.0,
        }
    }
}

/// Popover component - shows content positioned relative to trigger
///
/// Features:
/// - Click outside to close
/// - ESC key to close
/// - Positioning (top, bottom, left, right, etc.)
/// - Portal rendering (overlay)
/// - Animation (fade + zoom)
pub fn popover<T, C>(
    props: PopoverProps<T, C>,
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View
where
    T: IntoView + 'static,
    C: IntoView + 'static,
{
    let open = props.open;
    let position = props.position;
    let side_offset = props.side_offset;
    
    stack((
        // Trigger element
        container(props.trigger)
            .on_click_stop(move |_| {
                open.update(|o| *o = !*o);
            })
            .style(|s| s.cursor(CursorStyle::Pointer)),
        
        // Popover content (shown when open)
        container(
            container(props.content)
                .on_event_stop(EventListener::KeyDown, move |event| {
                    if let floem::event::Event::KeyDown(key_event) = event {
                        if key_event.key.logical_key == Key::Named(floem::keyboard::NamedKey::Escape) {
                            open.set(false);
                            return EventPropagation::Stop;
                        }
                    }
                    EventPropagation::Continue
                })
                .style(move |s| {
                    let cfg = config();
                    let is_open = open.get();
                    
                    if !is_open {
                        return s.display(floem::style::Display::None);
                    }
                    
                    // Base popover styles (matching Radix UI)
                    s.position(Position::Absolute)
                        .z_index(50)
                        .min_width(288.0) // w-72 = 18rem = 288px
                        .padding(16.0) // p-4
                        .border_radius(4.0) // rounded-xs
                        .background(cfg.color("panel.background"))
                        .border(1.0)
                        .border_color(cfg.color("lapce.border"))
                        .color(cfg.color("editor.foreground"))
                        // Shadow
                        .box_shadow_blur(8.0)
                        .box_shadow_color(cfg.color("editor.foreground").multiply_alpha(0.2))
                        // Positioning based on position prop
                        .apply_if(position == PopoverPosition::Bottom, |s| {
                            s.inset_top(side_offset)
                        })
                        .apply_if(position == PopoverPosition::Top, |s| {
                            s.inset_bottom(side_offset)
                        })
                        .apply_if(position == PopoverPosition::Left, |s| {
                            s.inset_right(side_offset)
                        })
                        .apply_if(position == PopoverPosition::Right, |s| {
                            s.inset_left(side_offset)
                        })
                })
        )
        // Click outside to close overlay
        .on_click_stop(move |_| {
            open.set(false);
        })
        .style(move |s| {
            let is_open = open.get();
            if !is_open {
                s.display(floem::style::Display::None)
            } else {
                // Transparent overlay to catch clicks outside
                s.position(Position::Absolute)
                    .inset(0.0)
                    .z_index(40)
            }
        }),
    ))
    .style(|s| s.position(Position::Relative))
}

/// Simple popover builder for common case
pub fn simple_popover<C>(
    trigger_text: &'static str,
    content: C,
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View
where
    C: IntoView + 'static,
{
    let open = floem::reactive::create_rw_signal(false);
    
    popover(
        PopoverProps::new(
            floem::views::label(|| trigger_text.to_string())
                .style(move |s| {
                    let cfg = config();
                    s.padding(8.0)
                        .border_radius(4.0)
                        .background(cfg.color("lapce.button_primary"))
                        .color(cfg.color("editor.background"))
                        .cursor(CursorStyle::Pointer)
                }),
            content,
            open,
        ),
        config,
    )
}
