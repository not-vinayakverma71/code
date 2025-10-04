/// History Types - EXACT 1:1 Translation from TypeScript
/// Source: Codex/packages/types/src/history.ts
use serde::{Deserialize, Serialize};

/// HistoryItem - Direct translation from TypeScript
/// Lines 7-22 from history.ts
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryItem {
    pub id: String,
    pub number: u32,
    pub ts: u64,
    pub task: String,
    pub tokens_in: u32,
    pub tokens_out: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_writes: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_reads: Option<u32>,
    pub total_cost: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_favorited: Option<bool>, // kilocode_change
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_notfound: Option<bool>, // kilocode_change
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
}
