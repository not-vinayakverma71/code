/// Handler registration types
use std::sync::Arc;
use anyhow::Result;
use serde::{Deserialize, Serialize};

pub type HandlerFn = Arc<dyn Fn() -> Result<()> + Send + Sync>;

pub struct HandlerContext {
    pub id: String,
    pub name: String,
}

pub enum HandlerType {
    Command,
    Event,
    Request,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebviewMessage {
    pub command: String,
    pub data: Option<serde_json::Value>,
}
