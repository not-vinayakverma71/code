// Task/Todo Tool Renderers
// Ported from UpdateTodoListToolBlock.tsx and task-related ChatRow cases

use std::sync::Arc;

use floem::{
    views::{Decorators, container, dyn_stack, h_stack, label, v_stack},
    View,
};

use crate::config::LapceConfig;

#[derive(Clone, Debug)]
pub struct TodoItem {
    pub text: String,
    pub completed: bool,
}

/// UpdateTodoList tool renderer
pub struct UpdateTodoListToolProps {
    pub todos: Vec<TodoItem>,
    pub content: Option<String>,
}

pub fn update_todo_list_tool(
    props: UpdateTodoListToolProps,
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    let todos = props.todos.clone();
    
    v_stack((
        // Header
        container(
            h_stack((
                label(|| "✓".to_string())
                    .style(|s| s.margin_right(8.0)),
                label(|| "Task List".to_string())
                    .style(|s| s),
            ))
        )
        .style(move |s| {
            let cfg = config();
            s.padding(8.0)
                .color(cfg.color("editor.foreground"))
        }),
        
        // Todo items
        container(
            dyn_stack(
                move || todos.clone(),
                |todo| (todo.text.clone(), todo.completed),
                move |todo| {
                    let completed = todo.completed;
                    let text = todo.text.clone();
                    h_stack((
                        label(move || if completed { "☑" } else { "☐" }.to_string())
                            .style(|s| s.margin_right(8.0)),
                        label(move || text.clone())
                            .style(|s| s),
                    ))
                    .style(move |s| {
                        let cfg = config();
                        s.padding(4.0)
                            .color(if completed {
                                cfg.color("editor.dim")
                            } else {
                                cfg.color("editor.foreground")
                            })
                    })
                }
            )
        )
        .style(|s| s.padding_left(24.0)),
    ))
    .style(|s| s.width_full())
}

/// NewTask tool renderer
pub struct NewTaskToolProps {
    pub task_description: String,
    pub mode: Option<String>,
}

pub fn new_task_tool(
    props: NewTaskToolProps,
    config: impl Fn() -> Arc<LapceConfig> + 'static + Copy,
) -> impl View {
    let description = props.task_description.clone();
    let mode = props.mode.clone();
    
    container(
        h_stack((
            label(|| "🎯".to_string())
                .style(|s| s.margin_right(8.0)),
            v_stack((
                label(|| "wants to create subtask".to_string())
                    .style(|s| s),
                label(move || description.clone())
                    .style(move |s| {
                        let cfg = config();
                        s.margin_top(4.0)
                            .color(cfg.color("editor.dim"))
                    }),
                if let Some(m) = mode {
                    label(move || format!("Mode: {}", m.clone()))
                        .style(move |s| {
                            let cfg = config();
                            s.margin_top(4.0)
                                .font_size(11.0)
                                .color(cfg.color("editor.dim"))
                        })
                } else {
                    label(|| "")
                },
            )),
        ))
    )
    .style(move |s| {
        let cfg = config();
        s.padding(8.0)
            .color(cfg.color("editor.foreground"))
            .width_full()
    })
}
