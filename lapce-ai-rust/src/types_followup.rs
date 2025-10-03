/// Followup Types - EXACT 1:1 Translation from TypeScript
/// Source: Codex/packages/types/src/followup.ts
use serde::{Deserialize, Serialize};

/// SuggestionItem - Direct translation from TypeScript
/// Lines 18-23 from followup.ts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuggestionItem {
    /// The text of the suggestion
    pub answer: String,
    /// Optional mode to switch to when selecting this suggestion
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
}

/// FollowUpData - Direct translation from TypeScript
/// Lines 8-13 from followup.ts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FollowUpData {
    /// The question being asked by the LLM
    #[serde(skip_serializing_if = "Option::is_none")]
    pub question: Option<String>,
    /// Array of suggested answers that the user can select
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggest: Option<Vec<SuggestionItem>>,
}
