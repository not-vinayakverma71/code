// Session Manager - Manage conversation sessions
// Create, switch, delete, and export chat sessions

use std::sync::Arc;
use floem::{
    reactive::{create_rw_signal, RwSignal, SignalGet, SignalUpdate},
    views::{container, h_stack, label, scroll, v_stack, Decorators},
    View,
};
use crate::config::LapceConfig;

#[derive(Debug, Clone)]
pub struct ChatSession {
    pub id: String,
    pub title: String,
    pub created_at: String,
    pub last_updated: String,
    pub message_count: usize,
    pub is_active: bool,
}

#[derive(Debug, Clone)]
pub struct SessionManagerData {
    pub sessions: Vec<ChatSession>,
    pub active_session_id: String,
}

/// Session manager component
/// Browse, switch, and manage chat sessions
pub fn session_manager(
    data: SessionManagerData,
    on_switch_session: impl Fn(String) + 'static + Copy,
    on_new_session: impl Fn() + 'static + Copy,
    on_delete_session: impl Fn(String) + 'static + Copy,
    on_export_session: impl Fn(String) + 'static + Copy,
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    let is_expanded = create_rw_signal(false); // Collapsed by default
    let confirm_delete = create_rw_signal::<Option<String>>(None);
    
    v_stack((
        // Header
        h_stack((
            label(move || if is_expanded.get() { "▼" } else { "▶" })
                .on_click_stop(move |_| {
                    is_expanded.update(|v| *v = !*v);
                })
                .style(move |s| {
                    let cfg = config();
                    s.color(cfg.color("editor.foreground"))
                        .font_size(12.0)
                        .margin_right(8.0)
                        .cursor(floem::style::CursorStyle::Pointer)
                }),
            
            label(|| "Sessions".to_string())
                .style(move |s| {
                    let cfg = config();
                    s.color(cfg.color("editor.foreground"))
                        .font_size(13.0)
                        .font_bold()
                        .flex_grow(1.0)
                }),
            
            // Session count
            label({
                let count = data.sessions.len();
                move || format!("{} sessions", count)
            })
            .style(move |s| {
                let cfg = config();
                s.color(cfg.color("editor.dim"))
                    .font_size(10.0)
                    .margin_right(12.0)
            }),
            
            // New session button
            container(
                label(|| "+".to_string())
            )
            .on_click_stop(move |_| {
                on_new_session();
            })
            .style(move |s| {
                let cfg = config();
                s.padding(4.0)
                    .padding_horiz(10.0)
                    .border_radius(3.0)
                    .background(cfg.color("lapce.button.primary.background"))
                    .color(cfg.color("lapce.button.primary.foreground"))
                    .font_size(14.0)
                    .font_bold()
                    .cursor(floem::style::CursorStyle::Pointer)
                    .hover(|s| s.background(cfg.color("panel.hovered.background")))
            }),
        ))
        .style(move |s| {
            let cfg = config();
            s.padding(12.0)
                .border_bottom(1.0)
                .border_color(cfg.color("lapce.border"))
                .background(cfg.color("panel.background"))
                .items_center()
        }),
        
        // Sessions list
        container(
            container(
                if !data.sessions.is_empty() {
                    scroll(
                    v_stack((
                        label({
                            let sessions = data.sessions.clone();
                            let active_id = data.active_session_id.clone();
                            move || sessions.iter().map(|session| {
                                let active_marker = if session.id == active_id {
                                    "► "
                                } else {
                                    "  "
                                };
                                
                                format!("{}{}\n   {} messages • Last: {}\n",
                                    active_marker,
                                    session.title,
                                    session.message_count,
                                    session.last_updated
                                )
                            }).collect::<Vec<_>>().join("\n")
                        })
                        .style(move |s| {
                            let cfg = config();
                            s.color(cfg.color("editor.foreground"))
                                .font_size(11.0)
                                .line_height(1.6)
                                .margin_bottom(16.0)
                        }),
                        
                        // Action buttons
                        h_stack((
                            container(
                                label(|| "Export Active".to_string())
                            )
                            .on_click_stop({
                                let active_id = data.active_session_id.clone();
                                move |_| {
                                    on_export_session(active_id.clone());
                                }
                            })
                            .style(move |s| {
                                let cfg = config();
                                s.padding(6.0)
                                    .padding_horiz(12.0)
                                    .border_radius(4.0)
                                    .background(cfg.color("panel.current.background"))
                                    .color(cfg.color("editor.foreground"))
                                    .font_size(11.0)
                                    .cursor(floem::style::CursorStyle::Pointer)
                                    .hover(|s| s.background(cfg.color("panel.hovered.background")))
                                    .margin_right(8.0)
                            }),
                            
                            container(
                                label(|| "Delete Active".to_string())
                            )
                            .on_click_stop({
                                let active_id = data.active_session_id.clone();
                                move |_| {
                                    confirm_delete.set(Some(active_id.clone()));
                                }
                            })
                            .style(move |s| {
                                let cfg = config();
                                s.padding(6.0)
                                    .padding_horiz(12.0)
                                    .border_radius(4.0)
                                    .background(cfg.color("list.errorForeground"))
                                    .color(cfg.color("editor.background"))
                                    .font_size(11.0)
                                    .cursor(floem::style::CursorStyle::Pointer)
                            }),
                        )),
                        
                        // Confirm delete dialog
                        container(
                            v_stack((
                                label(|| "⚠️ Delete session?".to_string())
                                    .style(move |s| {
                                        let cfg = config();
                                        s.color(cfg.color("list.errorForeground"))
                                            .font_size(12.0)
                                            .font_bold()
                                            .margin_bottom(8.0)
                                    }),
                                
                                label(|| "This action cannot be undone.".to_string())
                                    .style(move |s| {
                                        let cfg = config();
                                        s.color(cfg.color("editor.foreground"))
                                            .font_size(11.0)
                                            .margin_bottom(12.0)
                                    }),
                                
                                h_stack((
                                    container(
                                        label(|| "Cancel".to_string())
                                    )
                                    .on_click_stop(move |_| {
                                        confirm_delete.set(None);
                                    })
                                    .style(move |s| {
                                        let cfg = config();
                                        s.padding(6.0)
                                            .padding_horiz(12.0)
                                            .border_radius(4.0)
                                            .background(cfg.color("panel.current.background"))
                                            .color(cfg.color("editor.foreground"))
                                            .font_size(11.0)
                                            .cursor(floem::style::CursorStyle::Pointer)
                                            .margin_right(8.0)
                                    }),
                                    
                                    container(
                                        label(|| "Delete".to_string())
                                    )
                                    .on_click_stop(move |_| {
                                        if let Some(id) = confirm_delete.get() {
                                            on_delete_session(id);
                                            confirm_delete.set(None);
                                        }
                                    })
                                    .style(move |s| {
                                        let cfg = config();
                                        s.padding(6.0)
                                            .padding_horiz(12.0)
                                            .border_radius(4.0)
                                            .background(cfg.color("list.errorForeground"))
                                            .color(cfg.color("editor.background"))
                                            .font_size(11.0)
                                            .font_bold()
                                            .cursor(floem::style::CursorStyle::Pointer)
                                    }),
                                )),
                            ))
                        )
                        .style(move |s| {
                            let cfg = config();
                            let base = s
                                .padding(16.0)
                                .border(2.0)
                                .border_radius(6.0)
                                .border_color(cfg.color("list.errorForeground"))
                                .background(cfg.color("inputValidation.errorBackground"))
                                .margin_top(16.0);
                            
                            if confirm_delete.get().is_some() {
                                base
                            } else {
                                base.display(floem::style::Display::None)
                            }
                        }),
                    ))
                    .style(|s| s.padding(12.0))
                )
                .style(|s| s.max_height(500.0).width_full())
            } else {
                scroll(
                    container(
                        label(|| "No sessions\n\nClick + to create a new session".to_string())
                    )
                    .style(move |s| {
                        let cfg = config();
                        s.padding(24.0)
                            .color(cfg.color("editor.dim"))
                            .font_size(11.0)
                    })
                )
                .style(|s| s.max_height(500.0).width_full())
            }
            )
        )
        .style(move |s| {
            if is_expanded.get() {
                s
            } else {
                s.display(floem::style::Display::None)
            }
        }),
    ))
    .style(move |s| {
        let cfg = config();
        s.width_full()
            .border(1.0)
            .border_radius(6.0)
            .border_color(cfg.color("lapce.border"))
            .background(cfg.color("editor.background"))
            .margin_bottom(16.0)
    })
}
