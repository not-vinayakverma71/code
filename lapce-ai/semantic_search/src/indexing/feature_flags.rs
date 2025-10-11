// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: Copyright The LanceDB Authors

//! Feature flags for incremental indexing (CST-B12)
//!
//! Provides runtime toggles for:
//! - Canonical mapping only vs full CstApi
//! - Stable ID generation strategies
//! - Cache persistence options
//! - Performance optimizations

use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Serialize, Deserialize};

/// Feature flags for incremental indexing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureFlags {
    /// CST processing level
    pub cst_level: CstLevel,
    
    /// Stable ID generation strategy
    pub stable_id_strategy: StableIdStrategy,
    
    /// Cache persistence mode
    pub cache_mode: CacheMode,
    
    /// Enable parallel processing
    pub enable_parallel: bool,
    
    /// Enable batch embedding
    pub enable_batching: bool,
    
    /// Enable prefetching
    pub enable_prefetch: bool,
    
    /// Fallback behavior on errors
    pub fallback_behavior: FallbackBehavior,
}

/// CST processing level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CstLevel {
    /// Disabled - use legacy tree-sitter parsing
    Disabled,
    
    /// Canonical mapping only (Phase A)
    /// - Parse to tree-sitter Tree
    /// - Apply canonical kind mapping
    /// - No stable IDs
    MappingOnly,
    
    /// Full CstApi with stable IDs (Phase B)
    /// - Parse via CstApi
    /// - Generate stable IDs
    /// - Enable incremental indexing
    Full,
    
    /// Adaptive - start with MappingOnly, upgrade to Full if beneficial
    Adaptive,
}

/// Stable ID generation strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StableIdStrategy {
    /// Content-based hashing
    ContentHash,
    
    /// Position-based IDs (legacy)
    PositionBased,
    
    /// Hybrid (content hash + position)
    Hybrid,
    
    /// Best available from upstream
    Auto,
}

/// Cache persistence mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CacheMode {
    /// Memory only - no persistence
    MemoryOnly,
    
    /// Write-through to disk
    WriteThrough,
    
    /// Write-back (periodic sync)
    WriteBack,
    
    /// Phase4 tiered cache
    Tiered,
}

/// Fallback behavior on errors
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FallbackBehavior {
    /// Fail immediately
    FailFast,
    
    /// Fallback to legacy parsing
    UseLegacy,
    
    /// Retry with different strategy
    Retry,
    
    /// Skip file and log error
    Skip,
}

impl Default for FeatureFlags {
    fn default() -> Self {
        Self {
            cst_level: CstLevel::MappingOnly,
            stable_id_strategy: StableIdStrategy::Auto,
            cache_mode: CacheMode::MemoryOnly,
            enable_parallel: true,
            enable_batching: true,
            enable_prefetch: false,
            fallback_behavior: FallbackBehavior::UseLegacy,
        }
    }
}

impl FeatureFlags {
    /// Production configuration
    pub fn production() -> Self {
        Self {
            cst_level: CstLevel::Full,
            stable_id_strategy: StableIdStrategy::ContentHash,
            cache_mode: CacheMode::Tiered,
            enable_parallel: true,
            enable_batching: true,
            enable_prefetch: true,
            fallback_behavior: FallbackBehavior::UseLegacy,
        }
    }
    
    /// Development configuration
    pub fn development() -> Self {
        Self {
            cst_level: CstLevel::Adaptive,
            stable_id_strategy: StableIdStrategy::Auto,
            cache_mode: CacheMode::MemoryOnly,
            enable_parallel: false,
            enable_batching: true,
            enable_prefetch: false,
            fallback_behavior: FallbackBehavior::FailFast,
        }
    }
    
    /// Conservative configuration (minimal features)
    pub fn conservative() -> Self {
        Self {
            cst_level: CstLevel::MappingOnly,
            stable_id_strategy: StableIdStrategy::PositionBased,
            cache_mode: CacheMode::MemoryOnly,
            enable_parallel: false,
            enable_batching: false,
            enable_prefetch: false,
            fallback_behavior: FallbackBehavior::UseLegacy,
        }
    }
    
    /// Maximum performance configuration
    pub fn max_performance() -> Self {
        Self {
            cst_level: CstLevel::Full,
            stable_id_strategy: StableIdStrategy::Hybrid,
            cache_mode: CacheMode::Tiered,
            enable_parallel: true,
            enable_batching: true,
            enable_prefetch: true,
            fallback_behavior: FallbackBehavior::Retry,
        }
    }
    
    /// Check if stable IDs are enabled
    pub fn stable_ids_enabled(&self) -> bool {
        matches!(self.cst_level, CstLevel::Full | CstLevel::Adaptive)
    }
    
    /// Check if incremental indexing is available
    pub fn incremental_available(&self) -> bool {
        self.stable_ids_enabled() && 
        !matches!(self.cache_mode, CacheMode::MemoryOnly)
    }
    
    /// Check if canonical mapping is enabled
    pub fn mapping_enabled(&self) -> bool {
        !matches!(self.cst_level, CstLevel::Disabled)
    }
    
    /// Validate configuration
    pub fn validate(&self) -> Result<(), String> {
        // Full CstApi requires stable IDs
        if matches!(self.cst_level, CstLevel::Full) &&
           matches!(self.stable_id_strategy, StableIdStrategy::PositionBased) {
            return Err("Full CstApi requires content-based stable IDs".to_string());
        }
        
        // Tiered cache requires Full level
        if matches!(self.cache_mode, CacheMode::Tiered) &&
           !matches!(self.cst_level, CstLevel::Full) {
            return Err("Tiered cache requires CstLevel::Full".to_string());
        }
        
        Ok(())
    }
}

/// Thread-safe feature flag manager
pub struct FeatureFlagManager {
    flags: Arc<RwLock<FeatureFlags>>,
}

impl FeatureFlagManager {
    pub fn new(flags: FeatureFlags) -> Result<Self, String> {
        flags.validate()?;
        Ok(Self {
            flags: Arc::new(RwLock::new(flags)),
        })
    }
    
    pub fn default() -> Self {
        Self {
            flags: Arc::new(RwLock::new(FeatureFlags::default())),
        }
    }
    
    /// Get current flags (read-only copy)
    pub fn get(&self) -> FeatureFlags {
        self.flags.read().clone()
    }
    
    /// Update flags atomically
    pub fn update<F>(&self, updater: F) -> Result<(), String>
    where
        F: FnOnce(&mut FeatureFlags),
    {
        let mut flags = self.flags.write();
        updater(&mut *flags);
        flags.validate()?;
        Ok(())
    }
    
    /// Set CST level
    pub fn set_cst_level(&self, level: CstLevel) -> Result<(), String> {
        self.update(|f| f.cst_level = level)
    }
    
    /// Enable/disable parallel processing
    pub fn set_parallel(&self, enabled: bool) -> Result<(), String> {
        self.update(|f| f.enable_parallel = enabled)
    }
    
    /// Upgrade to full CstApi if adaptive
    pub fn try_upgrade_to_full(&self) -> bool {
        let mut flags = self.flags.write();
        if matches!(flags.cst_level, CstLevel::Adaptive) {
            flags.cst_level = CstLevel::Full;
            flags.validate().is_ok()
        } else {
            false
        }
    }
    
    /// Downgrade to mapping only
    pub fn downgrade_to_mapping(&self) -> Result<(), String> {
        self.update(|f| {
            f.cst_level = CstLevel::MappingOnly;
            if matches!(f.cache_mode, CacheMode::Tiered) {
                f.cache_mode = CacheMode::MemoryOnly;
            }
        })
    }
}

impl Default for FeatureFlagManager {
    fn default() -> Self {
        Self::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_default_flags() {
        let flags = FeatureFlags::default();
        assert!(flags.validate().is_ok());
        assert!(!flags.stable_ids_enabled());
        assert!(flags.mapping_enabled());
    }
    
    #[test]
    fn test_production_flags() {
        let flags = FeatureFlags::production();
        assert!(flags.validate().is_ok());
        assert!(flags.stable_ids_enabled());
        assert!(flags.incremental_available());
    }
    
    #[test]
    fn test_development_flags() {
        let flags = FeatureFlags::development();
        assert!(flags.validate().is_ok());
        assert!(flags.stable_ids_enabled()); // Adaptive can use stable IDs
    }
    
    #[test]
    fn test_conservative_flags() {
        let flags = FeatureFlags::conservative();
        assert!(flags.validate().is_ok());
        assert!(!flags.stable_ids_enabled());
        assert!(!flags.enable_parallel);
    }
    
    #[test]
    fn test_validation_errors() {
        // Full requires content-based IDs
        let mut flags = FeatureFlags::default();
        flags.cst_level = CstLevel::Full;
        flags.stable_id_strategy = StableIdStrategy::PositionBased;
        assert!(flags.validate().is_err());
        
        // Tiered cache requires Full
        let mut flags = FeatureFlags::default();
        flags.cache_mode = CacheMode::Tiered;
        assert!(flags.validate().is_err());
    }
    
    #[test]
    fn test_flag_manager() {
        let manager = FeatureFlagManager::default();
        let flags = manager.get();
        assert_eq!(flags.cst_level, CstLevel::MappingOnly);
        
        // Update level
        manager.set_cst_level(CstLevel::Full).unwrap();
        let flags = manager.get();
        assert_eq!(flags.cst_level, CstLevel::Full);
    }
    
    #[test]
    fn test_adaptive_upgrade() {
        let flags = FeatureFlags {
            cst_level: CstLevel::Adaptive,
            ..Default::default()
        };
        let manager = FeatureFlagManager::new(flags).unwrap();
        
        assert!(manager.try_upgrade_to_full());
        let flags = manager.get();
        assert_eq!(flags.cst_level, CstLevel::Full);
        
        // Can't upgrade again
        assert!(!manager.try_upgrade_to_full());
    }
    
    #[test]
    fn test_downgrade() {
        let flags = FeatureFlags::production();
        let manager = FeatureFlagManager::new(flags).unwrap();
        
        manager.downgrade_to_mapping().unwrap();
        let flags = manager.get();
        assert_eq!(flags.cst_level, CstLevel::MappingOnly);
        assert_eq!(flags.cache_mode, CacheMode::MemoryOnly);
    }
    
    #[test]
    fn test_parallel_toggle() {
        let manager = FeatureFlagManager::default();
        manager.set_parallel(false).unwrap();
        assert!(!manager.get().enable_parallel);
        
        manager.set_parallel(true).unwrap();
        assert!(manager.get().enable_parallel);
    }
    
    #[test]
    fn test_stable_id_checks() {
        let flags_mapping = FeatureFlags {
            cst_level: CstLevel::MappingOnly,
            ..Default::default()
        };
        assert!(!flags_mapping.stable_ids_enabled());
        
        let flags_full = FeatureFlags {
            cst_level: CstLevel::Full,
            ..Default::default()
        };
        assert!(flags_full.stable_ids_enabled());
        
        let flags_adaptive = FeatureFlags {
            cst_level: CstLevel::Adaptive,
            ..Default::default()
        };
        assert!(flags_adaptive.stable_ids_enabled());
    }
}
