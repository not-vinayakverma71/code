// Shortcuts Panel - Keyboard shortcuts reference
// Display available keyboard shortcuts for chat panel

use std::sync::Arc;
use floem::{
    reactive::{create_rw_signal, RwSignal, SignalGet, SignalUpdate},
    views::{container, h_stack, label, scroll, v_stack, Decorators},
    View,
};
use crate::config::LapceConfig;

#[derive(Debug, Clone)]
pub struct ShortcutItem {
    pub category: String,
    pub action: String,
    pub keys: String,
}

/// Shortcuts panel component
/// Shows keyboard shortcuts in a help dialog
pub fn shortcuts_panel(
    on_close: impl Fn() + 'static + Copy,
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    // Default shortcuts
    let shortcuts = vec![
        ShortcutItem {
            category: "Input".to_string(),
            action: "Send message".to_string(),
            keys: "Enter".to_string(),
        },
        ShortcutItem {
            category: "Input".to_string(),
            action: "New line".to_string(),
            keys: "Shift+Enter".to_string(),
        },
        ShortcutItem {
            category: "Input".to_string(),
            action: "Stop generation".to_string(),
            keys: "Esc".to_string(),
        },
        ShortcutItem {
            category: "Context".to_string(),
            action: "Attach file".to_string(),
            keys: "Ctrl+O".to_string(),
        },
        ShortcutItem {
            category: "Context".to_string(),
            action: "Attach folder".to_string(),
            keys: "Ctrl+Shift+O".to_string(),
        },
        ShortcutItem {
            category: "Context".to_string(),
            action: "Attach selection".to_string(),
            keys: "Ctrl+Shift+A".to_string(),
        },
        ShortcutItem {
            category: "Messages".to_string(),
            action: "Copy message".to_string(),
            keys: "Ctrl+C".to_string(),
        },
        ShortcutItem {
            category: "Messages".to_string(),
            action: "Edit message".to_string(),
            keys: "E".to_string(),
        },
        ShortcutItem {
            category: "Messages".to_string(),
            action: "Regenerate response".to_string(),
            keys: "Ctrl+R".to_string(),
        },
        ShortcutItem {
            category: "Navigation".to_string(),
            action: "Scroll to top".to_string(),
            keys: "Home".to_string(),
        },
        ShortcutItem {
            category: "Navigation".to_string(),
            action: "Scroll to bottom".to_string(),
            keys: "End".to_string(),
        },
        ShortcutItem {
            category: "Sessions".to_string(),
            action: "New session".to_string(),
            keys: "Ctrl+N".to_string(),
        },
        ShortcutItem {
            category: "Sessions".to_string(),
            action: "Switch session".to_string(),
            keys: "Ctrl+Tab".to_string(),
        },
        ShortcutItem {
            category: "Help".to_string(),
            action: "Show shortcuts".to_string(),
            keys: "Ctrl+/".to_string(),
        },
    ];
    
    v_stack((
        // Header
        h_stack((
            label(|| "Keyboard Shortcuts".to_string())
                .style(move |s| {
                    let cfg = config();
                    s.color(cfg.color("editor.foreground"))
                        .font_size(14.0)
                        .font_bold()
                        .flex_grow(1.0)
                }),
            
            container(
                label(|| "✕".to_string())
            )
            .on_click_stop(move |_| {
                on_close();
            })
            .style(move |s| {
                let cfg = config();
                s.padding(6.0)
                    .padding_horiz(10.0)
                    .border_radius(3.0)
                    .color(cfg.color("editor.foreground"))
                    .font_size(14.0)
                    .cursor(floem::style::CursorStyle::Pointer)
                    .hover(|s| s.background(cfg.color("list.hoverBackground")))
            }),
        ))
        .style(move |s| {
            let cfg = config();
            s.padding(16.0)
                .border_bottom(1.0)
                .border_color(cfg.color("lapce.border"))
                .background(cfg.color("titleBar.activeBackground"))
                .items_center()
        }),
        
        // Shortcuts list
        scroll(
            v_stack((
                label({
                    let mut current_category = String::new();
                    let mut output = String::new();
                    
                    for shortcut in shortcuts {
                        if shortcut.category != current_category {
                            if !output.is_empty() {
                                output.push_str("\n\n");
                            }
                            output.push_str(&format!("━━ {} ━━\n\n", shortcut.category));
                            current_category = shortcut.category.clone();
                        }
                        
                        output.push_str(&format!("{:<35} {}\n", shortcut.action, shortcut.keys));
                    }
                    
                    move || output.clone()
                })
                .style(move |s| {
                    let cfg = config();
                    s.color(cfg.color("editor.foreground"))
                        .font_family("monospace".to_string())
                        .font_size(11.0)
                        .line_height(1.8)
                }),
                
                // Footer note
                label(|| "\n💡 Tip: These shortcuts work when the chat panel is focused.".to_string())
                    .style(move |s| {
                        let cfg = config();
                        s.color(cfg.color("editor.dim"))
                            .font_size(11.0)
                            .margin_top(16.0)
                    }),
            ))
            .style(|s| s.padding(16.0))
        )
        .style(|s| s.flex_grow(1.0).width_full()),
        
        // Footer
        container(
            h_stack((
                label(|| "Press any key to close".to_string())
                    .style(move |s| {
                        let cfg = config();
                        s.color(cfg.color("editor.dim"))
                            .font_size(11.0)
                            .flex_grow(1.0)
                    }),
                
                container(
                    label(|| "Close".to_string())
                )
                .on_click_stop(move |_| {
                    on_close();
                })
                .style(move |s| {
                    let cfg = config();
                    s.padding(8.0)
                        .padding_horiz(16.0)
                        .border_radius(4.0)
                        .background(cfg.color("lapce.button.primary.background"))
                        .color(cfg.color("lapce.button.primary.foreground"))
                        .font_size(12.0)
                        .cursor(floem::style::CursorStyle::Pointer)
                }),
            ))
        )
        .style(move |s| {
            let cfg = config();
            s.padding(16.0)
                .border_top(1.0)
                .border_color(cfg.color("lapce.border"))
                .background(cfg.color("editor.background"))
        }),
    ))
    .style(move |s| {
        let cfg = config();
        s.width(600.0)
            .height(500.0)
            .border(1.0)
            .border_radius(8.0)
            .border_color(cfg.color("lapce.border"))
            .background(cfg.color("editor.background"))
    })
}

/// Quick shortcuts hint
/// Small inline hint showing common shortcuts
pub fn shortcuts_hint(
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    container(
        label(|| "💡 Enter to send • Shift+Enter for new line • Ctrl+/ for shortcuts".to_string())
    )
    .style(move |s| {
        let cfg = config();
        s.padding(8.0)
            .color(cfg.color("editor.dim"))
            .font_size(10.0)
            .width_full()
    })
}
