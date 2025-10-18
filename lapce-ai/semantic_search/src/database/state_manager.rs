// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: Copyright The LanceDB Authors
// Translation of state-manager.ts (Lines 1-116) - 100% EXACT

use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;

/// Line 3: IndexingState type definition
#[derive(Debug, Clone, PartialEq)]
pub enum IndexingState {
    Standby,
    Indexing,
    Indexed,
    Error,
}

// Re-export as IndexState for compatibility
pub type IndexState = IndexingState;

/// Lines 5-115: CodeIndexStateManager class
pub struct CodeIndexStateManager {
    // Lines 6-10: Private state fields
    system_status: Arc<Mutex<IndexingState>>,
    status_message: Arc<Mutex<String>>,
    processed_items: Arc<Mutex<usize>>,
    total_items: Arc<Mutex<usize>>,
    current_item_unit: Arc<Mutex<String>>,
    
    // Line 11: Progress emitter
    progress_sender: broadcast::Sender<StateUpdate>,
}

impl CodeIndexStateManager {
    /// Constructor - initializes state manager
    pub fn new() -> Self {
        let (progress_sender, _) = broadcast::channel(100);
        
        Self {
            system_status: Arc::new(Mutex::new(IndexingState::Standby)),
            status_message: Arc::new(Mutex::new(String::new())),
            processed_items: Arc::new(Mutex::new(0)),
            total_items: Arc::new(Mutex::new(0)),
            current_item_unit: Arc::new(Mutex::new("blocks".to_string())),
            progress_sender,
        }
    }
    
    /// Line 15: Public event for progress updates
    pub fn on_progress_update(&self) -> broadcast::Receiver<StateUpdate> {
        self.progress_sender.subscribe()
    }
    
    /// Lines 17-19: State getter
    pub fn state(&self) -> IndexingState {
        self.system_status.lock().unwrap().clone()
    }
    
    /// Lines 21-29: Get current status
    pub fn get_current_status(&self) -> StateUpdate {
        StateUpdate {
            system_status: self.system_status.lock().unwrap().clone(),
            message: self.status_message.lock().unwrap().clone(),
            processed_items: *self.processed_items.lock().unwrap(),
            total_items: *self.total_items.lock().unwrap(),
            current_item_unit: self.current_item_unit.lock().unwrap().clone(),
        }
    }
    
    /// Lines 33-56: Set system state
    pub fn set_system_state(&self, new_state: IndexingState, message: Option<String>) {
        let mut system_status = self.system_status.lock().unwrap();
        let mut status_message = self.status_message.lock().unwrap();
        
        // Line 34-35: Check if state changed
        let state_changed = *system_status != new_state || 
            (message.is_some() && message.as_ref().unwrap() != &*status_message);
        
        if state_changed {
            // Lines 38-41: Update state and message
            *system_status = new_state.clone();
            
            // Lines 44-52: Reset progress counters if not indexing
            if new_state != IndexingState::Indexing {
                *self.processed_items.lock().unwrap() = 0;
                *self.total_items.lock().unwrap() = 0;
                *self.current_item_unit.lock().unwrap() = "blocks".to_string();
            }
            
            // Handle message update
            if let Some(msg) = message {
                *status_message = msg;
            } else {
                // Set default messages for states if no message provided
                if new_state == IndexingState::Standby && new_state != IndexingState::Indexing {
                    *status_message = "Ready.".to_string();
                } else if new_state == IndexingState::Indexed && new_state != IndexingState::Indexing {
                    *status_message = "Index up-to-date.".to_string();
                } else if new_state == IndexingState::Error && new_state != IndexingState::Indexing {
                    *status_message = "An error occurred.".to_string();
                }
            }
            
            // Line 54: Fire progress update
            drop(system_status);
            drop(status_message);
            let _ = self.progress_sender.send(self.get_current_status());
        }
    }
    
    /// Lines 58-79: Report block indexing progress
    pub fn report_block_indexing_progress(&self, processed_items: usize, total_items: usize) {
        let mut proc_items = self.processed_items.lock().unwrap();
        let mut tot_items = self.total_items.lock().unwrap();
        let mut system_status = self.system_status.lock().unwrap();
        let mut status_message = self.status_message.lock().unwrap();
        let mut current_item_unit = self.current_item_unit.lock().unwrap();
        
        // Line 59: Check if progress changed
        let progress_changed = *proc_items != processed_items || *tot_items != total_items;
        
        // Lines 62-78: Update if changed or not already indexing
        if progress_changed || *system_status != IndexingState::Indexing {
            let old_status = system_status.clone();
            let old_message = status_message.clone();
            
            // Lines 63-65: Update progress values
            *proc_items = processed_items;
            *tot_items = total_items;
            *current_item_unit = "blocks".to_string();
            
            // Line 67: Create progress message
            let message = format!("Indexed {} / {} {} found", processed_items, total_items, current_item_unit);
            
            // Lines 71-72: Update state and message
            *system_status = IndexingState::Indexing;
            *status_message = message.clone();
            
            // Lines 75-77: Fire update if changed
            if old_status != *system_status || old_message != *status_message || progress_changed {
                drop(proc_items);
                drop(tot_items);
                drop(system_status);
                drop(status_message);
                drop(current_item_unit);
                let _ = self.progress_sender.send(self.get_current_status());
            }
        }
    }
    
    /// Lines 81-110: Report file queue progress
    pub fn report_file_queue_progress(
        &self, 
        processed_files: usize, 
        total_files: usize, 
        current_file_basename: Option<&str>
    ) {
        let mut proc_items = self.processed_items.lock().unwrap();
        let mut tot_items = self.total_items.lock().unwrap();
        let mut system_status = self.system_status.lock().unwrap();
        let mut status_message = self.status_message.lock().unwrap();
        let mut current_item_unit = self.current_item_unit.lock().unwrap();
        
        // Line 82: Check if progress changed
        let progress_changed = *proc_items != processed_files || *tot_items != total_files;
        
        if progress_changed || *system_status != IndexingState::Indexing {
            let old_status = system_status.clone();
            let old_message = status_message.clone();
            
            // Lines 85-88: Update progress values
            *proc_items = processed_files;
            *tot_items = total_files;
            *current_item_unit = "files".to_string();
            *system_status = IndexingState::Indexing;
            
            // Lines 90-99: Create appropriate message
            let message = if total_files > 0 && processed_files < total_files {
                format!(
                    "Processing {} / {} {}. Current: {}",
                    processed_files,
                    total_files,
                    current_item_unit,
                    current_file_basename.unwrap_or("...")
                )
            } else if total_files > 0 && processed_files == total_files {
                format!("Finished processing {} {} from queue.", total_files, current_item_unit)
            } else {
                "File queue processed.".to_string()
            };
            
            // Line 104: Update message
            *status_message = message.clone();
            
            // Lines 106-108: Fire update if changed
            if old_status != *system_status || old_message != *status_message || progress_changed {
                drop(proc_items);
                drop(tot_items);
                drop(system_status);
                drop(status_message);
                drop(current_item_unit);
                let _ = self.progress_sender.send(self.get_current_status());
            }
        }
    }
    
    /// Lines 112-114: Dispose method
    pub fn dispose(&self) {
        // In Rust, the broadcast sender will be dropped automatically
        // No explicit disposal needed
    }
}

/// State update structure for progress events
#[derive(Debug, Clone)]
pub struct StateUpdate {
    pub system_status: IndexingState,
    pub message: String,
    pub processed_items: usize,
    pub total_items: usize,
    pub current_item_unit: String,
}
