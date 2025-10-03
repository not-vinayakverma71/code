/// VSCode Types - EXACT 1:1 Translation from TypeScript
/// Source: Codex/packages/types/src/vscode.ts
use serde::{Deserialize, Serialize};

/// CodeActionId - Direct translation from TypeScript
/// Lines 8-10 from vscode.ts
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CodeActionId {
    ExplainCode,
    FixCode,
    ImproveCode,
    AddToContext,
    NewTask,
}

/// CodeActionName - Direct translation from TypeScript
/// Line 12 from vscode.ts
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CodeActionName {
    EXPLAIN,
    FIX,
    IMPROVE,
    ADD_TO_CONTEXT,
    NEW_TASK,
}

/// TerminalActionId - Direct translation from TypeScript
/// Lines 18-20 from vscode.ts
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TerminalActionId {
    TerminalAddToContext,
    TerminalFixCommand,
    TerminalExplainCommand,
}

/// TerminalActionName - Direct translation from TypeScript
/// Line 22 from vscode.ts
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TerminalActionName {
    ADD_TO_CONTEXT,
    FIX,
    EXPLAIN,
}

/// CommandId - Direct translation from TypeScript
/// Lines 30-66 from vscode.ts
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CommandId {
    ActivationCompleted,
    
    PlusButtonClicked,
    PromptsButtonClicked,
    McpButtonClicked,
    
    HistoryButtonClicked,
    MarketplaceButtonClicked,
    PopoutButtonClicked,
    AccountButtonClicked,
    SettingsButtonClicked,
    
    OpenInNewTab,
    
    ShowHumanRelayDialog,
    RegisterHumanRelayCallback,
    UnregisterHumanRelayCallback,
    HandleHumanRelayResponse,
    
    NewTask,
    
    SetCustomStoragePath,
    ImportSettings,
    
    AcceptInput,
    ProfileButtonClicked, // kilocode_change
    HelpButtonClicked, // kilocode_change
    FocusChatInput, // kilocode_change
    ExportSettings, // kilocode_change
    GenerateTerminalCommand, // kilocode_change
    FocusPanel,
}

/// Language - Direct translation from TypeScript
/// Lines 72-96 from vscode.ts
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Language {
    // From kiloLanguages
    Ar,
    Bg,
    Cs,
    Da,
    El,
    Et,
    Fi,
    He,
    Hu,
    Lv,
    Lt,
    Nb,
    Ro,
    Sk,
    Sl,
    Sv,
    Th,
    Uk,
    
    // Additional languages
    Ca,
    De,
    En,
    Es,
    Fr,
    Hi,
    Id,
    It,
    Ja,
    Ko,
    Nl,
    Pl,
    #[serde(rename = "pt-BR")]
    PtBR,
    Ru,
    Tr,
    Vi,
    #[serde(rename = "zh-CN")]
    ZhCN,
    #[serde(rename = "zh-TW")]
    ZhTW,
}

pub fn is_language(value: &str) -> bool {
    matches!(value,
        "ar" | "bg" | "cs" | "da" | "el" | "et" | "fi" | "he" | "hu" | "lv" | "lt" |
        "nb" | "ro" | "sk" | "sl" | "sv" | "th" | "uk" | "ca" | "de" | "en" | "es" |
        "fr" | "hi" | "id" | "it" | "ja" | "ko" | "nl" | "pl" | "pt-BR" | "ru" |
        "tr" | "vi" | "zh-CN" | "zh-TW")
}
