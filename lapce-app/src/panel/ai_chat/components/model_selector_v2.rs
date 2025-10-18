// Model Selector Dropdown - EXACT Windsurf model selection from real HTML
// Popover with search, grouped models, cost badges, "New" tags

use std::sync::Arc;
use floem::{
    peniko::Color,
    reactive::{RwSignal, SignalGet, SignalUpdate},
    views::{Decorators, container, dyn_stack, empty, h_stack, label, svg, v_stack, text_input, scroll},
    IntoView, View,
};

use crate::{
    config::LapceConfig,
    panel::ai_chat::icons::*,
};

#[derive(Clone, Debug)]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
    pub cost_label: String,  // "Free", "1x", "2x", "1.5x", "0.5x"
    pub is_new: bool,
    pub is_selected: bool,
}

pub struct ModelSelectorProps {
    pub current_model: RwSignal<String>,
    pub available_models: Vec<ModelInfo>,
    pub is_open: RwSignal<bool>,
}

/// Complete model selector with dropdown
pub fn model_selector_v2(
    props: ModelSelectorProps,
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    let current_model = props.current_model;
    let is_open = props.is_open;
    let search_query = RwSignal::new(String::new());
    
    container(
        v_stack((
            // Current model button
            model_button(current_model, is_open, config),
            
            // Dropdown (shown when is_open) - FIXED positioned to escape clipping
            container(
                scroll(
                    model_dropdown(search_query, current_model, is_open, config)
                )
                .style(|s| s.max_height(400.0))  // Scrollable if too tall
            )
            .style(move |s| {
                if is_open.get() {
                    // Use Absolute but with larger z-index and ensure it renders on top
                    s.position(floem::style::Position::Absolute)
                        .inset_top(40.0)  // Below the button
                        .inset_left(0.0)
                        .min_width(280.0)  // Wider for better visibility
                        .z_index(99999)  // Very high to render above everything
                } else {
                    s.display(floem::style::Display::None)
                }
            }),
        ))
        .style(|s| s.flex_col())
    )
    .style(|s| {
        s.position(floem::style::Position::Relative)  // Relative container for absolute child
    })
}

/// Model selector button (collapsed state)
fn model_button(
    current_model: RwSignal<String>,
    is_open: RwSignal<bool>,
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    container(
        h_stack((
            label(move || current_model.get())
                .style(move |s| {
                    let cfg = config();
                    s.font_size(12.0)
                        .color(cfg.color("editor.foreground"))
                }),
            
            // Chevron icon
            svg(|| ICON_CHEVRON_RIGHT.to_string())
                .style(move |s| {
                    let cfg = config();
                    let rotation = if is_open.get() { 90.0 } else { 0.0 };
                    s.width(10.0)
                        .height(10.0)
                        .color(cfg.color("editor.foreground").multiply_alpha(0.7))
                        .margin_left(4.0)
                        // TODO: Add rotation transform
                }),
        ))
        .style(|s| s.items_center())
    )
    .on_click_stop(move |_| {
        is_open.update(|open| *open = !*open);
    })
    .style(move |s| {
        let cfg = config();
        s.padding_horiz(8.0)
            .padding_vert(4.0)
            .border_radius(4.0)
            .cursor(floem::style::CursorStyle::Pointer)
            .hover(|s| {
                let cfg = config();
                s.background(cfg.color("editor.foreground").multiply_alpha(0.1))
            })
    })
}

/// Model dropdown menu - EXACT copy from Windsurf HTML
fn model_dropdown(
    search_query: RwSignal<String>,
    current_model: RwSignal<String>,
    is_open: RwSignal<bool>,
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    // Outer container: p-[6px] rounded-[12px] max-h-[300px] w-60
    v_stack((
        // Search header: flex items-center justify-between rounded-[6px] bg-neutral-500/10 p-1
        container(
            // Inner wrapper: flex h-5 w-full items-center gap-1
            container(
                h_stack((
                    svg(|| ICON_SEARCH.to_string())
                        .style(move |s| {
                            let cfg = config();
                            s.width(14.0)  // size-3.5
                                .height(14.0)
                                .color(cfg.color("editor.foreground").multiply_alpha(0.7))
                        }),
                    text_input(search_query)
                        .placeholder("Search all models".to_string())
                        .style(move |s| {
                            let cfg = config();
                            s.flex_grow(1.0)
                                .font_size(12.0)  // text-xs
                                .background(Color::TRANSPARENT)
                                .border(0.0)
                                .color(cfg.color("editor.foreground"))
                        }),
                    // Group By button
                    container(
                        h_stack((
                            svg(|| r#"<svg xmlns='http://www.w3.org/2000/svg' width='24' height='24' viewBox='0 0 24 24' fill='none' stroke='currentColor' stroke-width='2'><path d='M3 6h18'></path><path d='M7 12h10'></path><path d='M10 18h4'></path></svg>"#.to_string())
                                .style(move |s| {
                                    let cfg = config();
                                    s.width(12.0).height(12.0).color(cfg.color("editor.foreground"))
                                }),
                            label(|| "Group By".to_string()).style(|s| s.font_size(12.0)),
                        ))
                        .style(|s| s.items_center().gap(4.0))
                    )
                    .style(move |s| {
                        let cfg = config();
                        s.padding_horiz(4.0)
                            .border_radius(4.0)
                            .color(cfg.color("editor.foreground").multiply_alpha(0.5))
                            .cursor(floem::style::CursorStyle::Pointer)
                            .hover(|s| {
                                let cfg = config();
                                s.background(cfg.color("editor.foreground").multiply_alpha(0.2))
                            })
                    }),
                    svg(|| r#"<svg xmlns='http://www.w3.org/2000/svg' width='24' height='24' viewBox='0 0 24 24' fill='none' stroke='currentColor' stroke-width='2'><circle cx='12' cy='12' r='10'></circle><path d='M9.09 9a3 3 0 0 1 5.83 1c0 2-3 3-3 3'></path><path d='M12 17h.01'></path></svg>"#.to_string())
                        .style(move |s| {
                            let cfg = config();
                            s.width(12.0).height(12.0).color(cfg.color("editor.foreground").multiply_alpha(0.5))
                        }),
                ))
                .style(|s| {
                    s.flex_row()
                        .items_center()
                        .gap(4.0)  // gap-1
                        .height(20.0)  // h-5
                        .width_full()
                })
            )
        )
        .style(move |s| {
            let cfg = config();
            s.flex_row()
                .items_center()
                .justify_between()
                .border_radius(6.0)  // rounded-[6px]
                .background(cfg.color("editor.foreground").multiply_alpha(0.1))  // bg-neutral-500/10
                .padding(4.0)  // p-1
        }),
        
        // Content area: flex flex-grow flex-col justify-between py-1
        v_stack((
                // Model sections: space-y-2
                container(
                    v_stack((
                        model_section(
                            "Recently Used",
                            recently_used_models(),
                            current_model,
                            is_open,
                            config,
                        ),
                        model_section(
                            "Recommended",
                            recommended_models(),
                            current_model,
                            is_open,
                            config,
                        ),
                    ))
                    .style(|s| s.flex_col().gap(8.0))  // space-y-2
                ),
                
                // See more button: self-start px-2 pb-1 pt-1 text-xs opacity-50
                container(
                    label(|| "See more".to_string())
                        .style(move |s| {
                            let cfg = config();
                            s.font_size(12.0)  // text-xs
                                .color(cfg.color("editor.foreground").multiply_alpha(0.5))
                        })
                )
                .style(move |s| {
                    let cfg = config();
                    s.padding_horiz(8.0)  // px-2
                        .padding_bottom(4.0)  // pb-1
                        .padding_top(4.0)  // pt-1
                        .cursor(floem::style::CursorStyle::Pointer)
                        .hover(|s| {
                            let cfg = config();
                            s.color(cfg.color("editor.foreground").multiply_alpha(0.8))
                        })
                }),
            ))
            .style(|s| s.flex_col().justify_between().padding_vert(4.0)),  // py-1
    ))
    .style(move |s| {
        let cfg = config();
        s.flex_col()
            .padding(6.0)  // p-[6px]
            .border_radius(12.0)  // rounded-[12px]
            .width_full()
            .border(1.0)
            .border_color(cfg.color("lapce.border"))
            .background(cfg.color("panel.background"))
            .box_shadow_blur(8.0)
            .box_shadow_color(Color::BLACK.multiply_alpha(0.2))
    })
}

/// Model section with header - EXACT from HTML
/// HTML: <div><div class="flex select-none items-center gap-1 px-2 py-1 text-xs font-medium opacity-50">Title</div><div>items</div></div>
fn model_section(
    title: &'static str,
    models: Vec<ModelInfo>,
    current_model: RwSignal<String>,
    is_open: RwSignal<bool>,
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    v_stack((
        // Section header: flex select-none items-center gap-1 px-2 py-1 text-xs font-medium opacity-50
        container(
            label(|| title.to_string())
                .style(move |s| {
                    let cfg = config();
                    s.font_size(12.0)  // text-xs
                        .font_weight(floem::text::Weight::MEDIUM)
                        .color(cfg.color("editor.foreground").multiply_alpha(0.5))
                })
        )
        .style(|s| {
            s.flex_row()
                .items_center()
                .gap(4.0)  // gap-1
                .padding_horiz(8.0)  // px-2
                .padding_vert(4.0)  // py-1
        }),
        
        // Model items - VERTICAL STACK (each model on its own row)
        dyn_stack(
            move || models.clone(),
            |model| model.id.clone(),
            move |model| {
                model_item_windsurf(model, current_model, is_open, config)
            }
        )
    ))
    .style(|s| s.flex_col())
}

/// Individual model item - EXACT from HTML
/// HTML: <div class="cursor-default select-none list-none text-xs"><button class="flex w-full justify-between gap-1 rounded-[6px] px-2 py-1 hover:bg-neutral-500/10">...</button></div>
fn model_item_windsurf(
    model: ModelInfo,
    current_model: RwSignal<String>,
    is_open: RwSignal<bool>,
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    let model_id = model.id.clone();
    let model_id_click = model.id.clone();
    let model_name = model.name.clone();
    let cost_label = model.cost_label.clone();
    let is_new = model.is_new;
    let is_selected = model.is_selected;
    
    // Outer div: cursor-default select-none list-none text-xs
    container(
        // Inner button: flex w-full justify-between gap-1 rounded-[6px] px-2 py-1
        container(
            h_stack((
                // Left: flex items-center gap-x-1 overflow-hidden
                h_stack((
                    // Model name: overflow-hidden text-ellipsis whitespace-nowrap text-xs
                    label(move || model_name.clone())
                        .style(move |s| {
                            let cfg = config();
                            s.font_size(12.0)  // text-xs
                                .color(cfg.color("editor.foreground"))
                        }),
                    
                    // "New" badge: rounded-full bg-green-500/15 px-1.5 text-xs
                    if is_new {
                        container(
                            label(|| "New".to_string())
                                .style(move |s| {
                                    let cfg = config();
                                    s.font_size(12.0)  // text-xs
                                        .color(cfg.color("editor.foreground"))
                                })
                        )
                        .style(|s| {
                            s.padding_horiz(6.0)  // px-1.5
                                .border_radius(999.0)  // rounded-full
                                .background(Color::from_rgb8(34, 197, 94).multiply_alpha(0.15))  // bg-green-500/15
                        })
                        .into_any()
                    } else {
                        empty().into_any()
                    },
                ))
                .style(|s| {
                    s.flex_row()
                        .items_center()
                        .gap(4.0)  // gap-x-1
                }),
                
                // Right: flex items-center gap-1
                h_stack((
                    // Cost label: whitespace-nowrap rounded text-xs opacity-50
                    container(
                        label(move || cost_label.clone())
                            .style(move |s| {
                                let cfg = config();
                                s.font_size(12.0)  // text-xs
                                    .color(cfg.color("editor.foreground").multiply_alpha(0.5))
                            })
                    )
                    .style(|s| s.border_radius(4.0)),
                    
                    // Checkmark: size-3 opacity-50
                    if is_selected {
                        svg(|| r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M20 6 9 17l-5-5"></path></svg>"#.to_string())
                            .style(move |s| {
                                let cfg = config();
                                s.width(12.0)  // size-3
                                    .height(12.0)
                                    .color(cfg.color("editor.foreground").multiply_alpha(0.5))
                            })
                            .into_any()
                    } else {
                        empty().into_any()
                    },
                ))
                .style(|s| {
                    s.flex_row()
                        .items_center()
                        .gap(4.0)  // gap-1
                }),
            ))
            .style(|s| {
                s.flex_row()
                    .width_full()
                    .justify_between()
                    .gap(4.0)  // gap-1
            })
        )
        .on_click_stop(move |_| {
            current_model.set(model_id_click.clone());
            is_open.set(false);
        })
        .style(move |s| {
            let cfg = config();
            s.flex_row()
                .width_full()
                .justify_between()
                .gap(4.0)
                .border_radius(6.0)  // rounded-[6px]
                .padding_horiz(8.0)  // px-2
                .padding_vert(4.0)  // py-1
                .cursor(floem::style::CursorStyle::Pointer)
                .hover(|s| {
                    let cfg = config();
                    s.background(cfg.color("editor.foreground").multiply_alpha(0.1))  // hover:bg-neutral-500/10
                })
        })
    )
    .style(move |s| {
        let cfg = config();
        s.font_size(12.0)  // text-xs
            .color(cfg.color("editor.foreground"))
    })
}

/// Recently used models (EXACT from Windsurf HTML)
pub fn recently_used_models() -> Vec<ModelInfo> {
    vec![
        ModelInfo {
            id: "claude-sonnet-4.5-thinking".to_string(),
            name: "Claude Sonnet 4.5 Thinking (promo)".to_string(),
            cost_label: "1.5x".to_string(),
            is_new: false,
            is_selected: true,
        },
        ModelInfo {
            id: "gpt-5-codex".to_string(),
            name: "GPT-5-Codex".to_string(),
            cost_label: "Free".to_string(),
            is_new: true,
            is_selected: false,
        },
        ModelInfo {
            id: "gpt-5-high".to_string(),
            name: "GPT-5 (high reasoning)".to_string(),
            cost_label: "2x".to_string(),
            is_new: true,
            is_selected: false,
        },
    ]
}

/// Recommended models (EXACT from Windsurf HTML)
pub fn recommended_models() -> Vec<ModelInfo> {
    vec![
        ModelInfo {
            id: "claude-sonnet-4.5-promo".to_string(),
            name: "Claude Sonnet 4.5 (promo)".to_string(),
            cost_label: "1x".to_string(),
            is_new: false,
            is_selected: false,
        },
        ModelInfo {
            id: "gpt-5-low".to_string(),
            name: "GPT-5 (low reasoning)".to_string(),
            cost_label: "0.5x".to_string(),
            is_new: true,
            is_selected: false,
        },
        ModelInfo {
            id: "claude-sonnet-4".to_string(),
            name: "Claude Sonnet 4".to_string(),
            cost_label: "2x".to_string(),
            is_new: false,
            is_selected: false,
        },
        ModelInfo {
            id: "gemini-2.5-pro".to_string(),
            name: "Gemini 2.5 Pro".to_string(),
            cost_label: "1x".to_string(),
            is_new: false,
            is_selected: false,
        },
        ModelInfo {
            id: "code-supernova".to_string(),
            name: "code-supernova-1-million".to_string(),
            cost_label: "Free".to_string(),
            is_new: true,
            is_selected: false,
        },
    ]
}
