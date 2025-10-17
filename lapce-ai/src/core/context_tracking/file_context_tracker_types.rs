//! File Context Tracker Types
//!
//! Direct 1:1 port from Codex/src/core/context-tracking/FileContextTrackerTypes.ts
//! Lines 1-29 complete
//!
//! Zod schemas translated to Rust serde types with validation.

use serde::{Deserialize, Serialize};

/// Source of a file context record
///
/// Port of recordSourceSchema from FileContextTrackerTypes.ts line 4
/// Extended for tool integration (PORT-CT-25)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecordSource {
    ReadTool,
    WriteTool,
    DiffApply,
    Mention,
    UserEdited,
    RooEdited,
    FileMentioned,
}

/// State of a file context record
///
/// Port of record_state enum from FileContextTrackerTypes.ts line 12
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecordState {
    Active,
    Stale,
}

/// Metadata entry for a single file
///
/// Port of fileMetadataEntrySchema from FileContextTrackerTypes.ts lines 10-17
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMetadataEntry {
    pub path: String,
    pub record_state: RecordState,
    pub record_source: RecordSource,
    pub roo_read_date: Option<u64>,
    pub roo_edit_date: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_edit_date: Option<u64>,
}

/// Task metadata containing all files in context
///
/// Port of taskMetadataSchema from FileContextTrackerTypes.ts lines 23-25
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskMetadata {
    pub files_in_context: Vec<FileMetadataEntry>,
}

impl Default for TaskMetadata {
    fn default() -> Self {
        Self {
            files_in_context: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_record_source_serialization() {
        let sources = vec![
            RecordSource::ReadTool,
            RecordSource::WriteTool,
            RecordSource::DiffApply,
            RecordSource::Mention,
            RecordSource::UserEdited,
            RecordSource::RooEdited,
            RecordSource::FileMentioned,
        ];
        
        for source in sources {
            let json = serde_json::to_string(&source).unwrap();
            let deserialized: RecordSource = serde_json::from_str(&json).unwrap();
            assert_eq!(source, deserialized);
        }
    }
    
    #[test]
    fn test_record_state_serialization() {
        let states = vec![RecordState::Active, RecordState::Stale];
        
        for state in states {
            let json = serde_json::to_string(&state).unwrap();
            let deserialized: RecordState = serde_json::from_str(&json).unwrap();
            assert_eq!(state, deserialized);
        }
    }
    
    #[test]
    fn test_file_metadata_entry_serialization() {
        let entry = FileMetadataEntry {
            path: "src/main.rs".to_string(),
            record_state: RecordState::Active,
            record_source: RecordSource::ReadTool,
            roo_read_date: Some(1234567890),
            roo_edit_date: None,
            user_edit_date: Some(1234567900),
        };
        
        let json = serde_json::to_string(&entry).unwrap();
        let deserialized: FileMetadataEntry = serde_json::from_str(&json).unwrap();
        
        assert_eq!(entry.path, deserialized.path);
        assert_eq!(entry.record_state, deserialized.record_state);
        assert_eq!(entry.roo_read_date, deserialized.roo_read_date);
    }
    
    #[test]
    fn test_task_metadata_default() {
        let metadata = TaskMetadata::default();
        assert_eq!(metadata.files_in_context.len(), 0);
    }
}
