/// WebviewMessage type for handler registration
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebviewMessage {
    #[serde(rename = "type")]
    pub msg_type: String,
    pub text: Option<String>,
    pub data: Option<serde_json::Value>,
}
