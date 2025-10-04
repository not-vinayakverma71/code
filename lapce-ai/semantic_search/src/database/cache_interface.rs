// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: Copyright The LanceDB Authors
// Translation of interfaces/cache.ts (Lines 1-7) - 100% EXACT

use std::collections::HashMap;

/// Lines 1-6: ICacheManager interface
pub trait ICacheManager: Send + Sync {
    /// Line 2: Get hash for a file path
    fn get_hash(&self, file_path: &str) -> Option<String>;
    
    /// Line 3: Update hash for a file path
    fn update_hash(&self, file_path: &str, hash: String);
    
    /// Line 4: Delete hash for a file path
    fn delete_hash(&self, file_path: &str);
    
    /// Line 5: Get all hashes
    fn get_all_hashes(&self) -> HashMap<String, String>;
}
