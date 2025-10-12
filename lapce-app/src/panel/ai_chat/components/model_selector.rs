// Model Selector - Using Floem's built-in Dropdown
// Replaces complex Radix UI port with simple Floem component

use std::sync::Arc;

use floem::{
    reactive::{RwSignal, SignalGet, SignalUpdate},
    views::{dropdown::Dropdown, h_stack, label, Decorators},
    IntoView, View,
};

use crate::config::LapceConfig;

#[derive(Debug, Clone, PartialEq)]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
    pub provider: String,
    pub context_window: usize,
}

impl std::fmt::Display for ModelInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({})", self.name, self.provider)
    }
}

/// Get available models (placeholder - will come from backend/IPC later)
pub fn get_available_models() -> Vec<ModelInfo> {
    vec![
        ModelInfo {
            id: "gpt-4".to_string(),
            name: "GPT-4".to_string(),
            provider: "OpenAI".to_string(),
            context_window: 8192,
        },
        ModelInfo {
            id: "gpt-4-turbo".to_string(),
            name: "GPT-4 Turbo".to_string(),
            provider: "OpenAI".to_string(),
            context_window: 128000,
        },
        ModelInfo {
            id: "claude-3-opus".to_string(),
            name: "Claude 3 Opus".to_string(),
            provider: "Anthropic".to_string(),
            context_window: 200000,
        },
        ModelInfo {
            id: "claude-3-sonnet".to_string(),
            name: "Claude 3 Sonnet".to_string(),
            provider: "Anthropic".to_string(),
            context_window: 200000,
        },
        ModelInfo {
            id: "gemini-pro".to_string(),
            name: "Gemini Pro".to_string(),
            provider: "Google".to_string(),
            context_window: 32000,
        },
    ]
}

/// Model selector dropdown component
pub fn model_selector(
    selected_model: RwSignal<ModelInfo>,
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    let models = get_available_models();
    
    h_stack((
        label(|| "Model:".to_string())
            .style(move |s| {
                let cfg = config();
                s.margin_right(8.0)
                    .color(cfg.color("editor.foreground"))
                    .font_size(13.0)
            }),
        
        Dropdown::new_rw(selected_model, models.into_iter())
            .style(move |s| {
                let cfg = config();
                s.min_width(250.0)
                    .padding(6.0)
                    .padding_horiz(12.0)
                    .border_radius(4.0)
                    .background(cfg.color("panel.background"))
                    .border(1.0)
                    .border_color(cfg.color("lapce.border"))
                    .color(cfg.color("editor.foreground"))
            }),
    ))
    .style(|s| s.items_center().gap(8.0))
}

/// Compact model selector for toolbar
pub fn model_selector_compact(
    selected_model: RwSignal<ModelInfo>,
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    let models = get_available_models();
    
    Dropdown::new_rw(selected_model, models.into_iter())
        .style(move |s| {
            let cfg = config();
            s.min_width(180.0)
                .padding(4.0)
                .padding_horiz(10.0)
                .border_radius(4.0)
                .background(cfg.color("panel.background"))
                .border(1.0)
                .border_color(cfg.color("lapce.border"))
                .color(cfg.color("editor.foreground"))
                .font_size(12.0)
        })
}

/// Model selector with info display
pub fn model_selector_with_info(
    selected_model: RwSignal<ModelInfo>,
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    let models = get_available_models();
    
    floem::views::v_stack((
        h_stack((
            label(|| "Model:".to_string())
                .style(move |s| {
                    let cfg = config();
                    s.margin_right(8.0)
                        .color(cfg.color("editor.foreground"))
                        .font_size(13.0)
                }),
            
            Dropdown::new_rw(selected_model, models.into_iter())
                .style(move |s| {
                    let cfg = config();
                    s.min_width(250.0)
                        .padding(6.0)
                        .padding_horiz(12.0)
                        .border_radius(4.0)
                        .background(cfg.color("panel.background"))
                        .border(1.0)
                        .border_color(cfg.color("lapce.border"))
                        .color(cfg.color("editor.foreground"))
                }),
        ))
        .style(|s| s.items_center().gap(8.0)),
        
        // Model info
        label(move || {
            let model = selected_model.get();
            format!("Context: {} tokens", model.context_window)
        })
        .style(move |s| {
            let cfg = config();
            s.margin_top(4.0)
                .font_size(11.0)
                .color(cfg.color("editor.dim"))
        }),
    ))
    .style(|s| s.flex_col())
}
