// Approval Dialog Components
// Pre-IPC Phase 8: UI-only approval interface

use std::rc::Rc;
use std::sync::Arc;

use floem::{
    reactive::SignalGet,
    views::{Decorators, container, h_stack, label, v_stack},
    View,
};

use crate::config::LapceConfig;

pub struct ApprovalDialogProps {
    pub title: String,
    pub message: String,
    pub on_approve: Rc<dyn Fn()>,
    pub on_deny: Rc<dyn Fn()>,
    pub show_always_allow: bool,
}

/// Simple approval dialog for tool requests
/// TODO: Wire to actual approval system when IPC ready
pub fn approval_dialog(
    props: ApprovalDialogProps,
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    let title = props.title.clone();
    let message = props.message.clone();
    let on_approve = props.on_approve.clone();
    let on_deny = props.on_deny.clone();
    
    container(
        v_stack((
            // Title
            label(move || title.clone())
                .style(move |s| {
                    let cfg = config();
                    s.padding_bottom(8.0)
                        .font_size(14.0)
                        .color(cfg.color("editor.foreground"))
                }),
            
            // Message
            label(move || message.clone())
                .style(move |s| {
                    let cfg = config();
                    s.padding_bottom(12.0)
                        .color(cfg.color("editor.dim"))
                }),
            
            // Actions
            h_stack((
                label(|| "Deny".to_string())
                    .on_click_stop(move |_| on_deny())
                    .style(move |s| {
                        let cfg = config();
                        s.padding_horiz(16.0)
                            .padding_vert(8.0)
                            .background(cfg.color("lapce.error"))
                            .border_radius(4.0)
                            .cursor(floem::style::CursorStyle::Pointer)
                    }),
                label(|| "Approve".to_string())
                    .on_click_stop(move |_| on_approve())
                    .style(move |s| {
                        let cfg = config();
                        s.padding_horiz(16.0)
                            .padding_vert(8.0)
                            .background(cfg.color("lapce.button_primary"))
                            .border_radius(4.0)
                            .cursor(floem::style::CursorStyle::Pointer)
                    }),
            ))
            .style(|s| s.gap(8.0).justify_end()),
        ))
    )
    .style(move |s| {
        let cfg = config();
        s.padding(16.0)
            .background(cfg.color("panel.background"))
            .border(1.0)
            .border_color(cfg.color("lapce.border"))
            .border_radius(4.0)
            .width_full()
    })
}
