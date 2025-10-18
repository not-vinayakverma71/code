/// Experiment Types - EXACT 1:1 Translation from TypeScript
/// Source: Codex/packages/types/src/experiment.ts
use serde::{Deserialize, Serialize};

/// ExperimentId - Direct translation from TypeScript
/// Lines 9-14 from experiment.ts
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ExperimentId {
    // kilocode experiments
    MorphFastApply,
    // standard experiments
    PowerSteering,
    MultiFileApplyDiff,
    PreventFocusDisruption,
}

/// Experiments - Direct translation from TypeScript
/// Lines 20-27 from experiment.ts
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Experiments {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub morph_fast_apply: Option<bool>, // kilocode_change
    #[serde(skip_serializing_if = "Option::is_none")]
    pub power_steering: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub multi_file_apply_diff: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prevent_focus_disruption: Option<bool>,
}
