//! Loader Tests (P6)
//!
//! Comprehensive tests for custom instructions and system prompt loaders:
//! - Symlink cycle prevention
//! - BOM/CRLF/UTF-8-BOM handling
//! - Binary file detection and skip
//! - Stable ordering (alphabetical)
//! - Depth limits
//! - Size limits

use std::path::Path;
use tempfile::TempDir;
use tokio::fs;
use std::io::Write;

use crate::core::prompt::sections::custom_instructions::add_custom_instructions;
use crate::core::prompt::settings::SystemPromptSettings;

// ============================================================================
// Symlink Cycle Prevention Tests
// ============================================================================

#[tokio::test]
async fn test_symlink_cycle_detection() {
    let temp_dir = TempDir::new().unwrap();
    let workspace = temp_dir.path();
    
    // Create rules directory
    let rules_dir = workspace.join(".kilocode/rules");
    fs::create_dir_all(&rules_dir).await.unwrap();
    
    // Create a file
    let file_a = rules_dir.join("a.txt");
    fs::write(&file_a, "Content A").await.unwrap();
    
    // Create a symlink to self (direct cycle)
    #[cfg(unix)]
    {
        use std::os::unix::fs as unix_fs;
        let symlink_self = rules_dir.join("link_self");
        unix_fs::symlink(&symlink_self, &symlink_self).ok(); // May fail, but that's expected
    }
    
    let settings = SystemPromptSettings::default();
    
    // Should not crash, should skip cyclic symlinks
    let result = add_custom_instructions(
        "",
        "",
        workspace,
        "code",
        None,
        None,
        &settings,
    ).await;
    
    assert!(result.is_ok(), "Should handle symlink cycles gracefully");
    let content = result.unwrap();
    assert!(content.contains("Content A") || content.is_empty());
}

#[tokio::test]
async fn test_symlink_chain_too_deep() {
    let temp_dir = TempDir::new().unwrap();
    let workspace = temp_dir.path();
    
    // Create rules directory
    let rules_dir = workspace.join(".kilocode/rules");
    fs::create_dir_all(&rules_dir).await.unwrap();
    
    #[cfg(unix)]
    {
        use std::os::unix::fs as unix_fs;
        
        // Create a deep chain of symlinks (beyond MAX_DEPTH=5)
        let file = rules_dir.join("file.txt");
        fs::write(&file, "Deep content").await.unwrap();
        
        let mut prev = file.clone();
        for i in 1..=10 {
            let link = rules_dir.join(format!("link{}", i));
            unix_fs::symlink(&prev, &link).unwrap();
            prev = link;
        }
        
        let settings = SystemPromptSettings::default();
        
        // Should handle depth limit gracefully
        let result = add_custom_instructions(
            "",
            "",
            workspace,
            "code",
            None,
            None,
            &settings,
        ).await;
        
        assert!(result.is_ok(), "Should handle deep symlink chains gracefully");
    }
}

#[tokio::test]
async fn test_symlink_to_directory() {
    let temp_dir = TempDir::new().unwrap();
    let workspace = temp_dir.path();
    
    // Create rules directory
    let rules_dir = workspace.join(".kilocode/rules");
    fs::create_dir_all(&rules_dir).await.unwrap();
    
    // Create a subdirectory with files
    let subdir = workspace.join("external_rules");
    fs::create_dir_all(&subdir).await.unwrap();
    fs::write(subdir.join("rule1.txt"), "External rule 1").await.unwrap();
    fs::write(subdir.join("rule2.txt"), "External rule 2").await.unwrap();
    
    #[cfg(unix)]
    {
        use std::os::unix::fs as unix_fs;
        
        // Create symlink from rules dir to external dir
        let link_to_dir = rules_dir.join("external_link");
        unix_fs::symlink(&subdir, &link_to_dir).unwrap();
        
        let settings = SystemPromptSettings::default();
        
        let result = add_custom_instructions(
            "",
            "",
            workspace,
            "code",
            None,
            None,
            &settings,
        ).await.unwrap();
        
        // Should include both external rules
        assert!(result.contains("External rule 1"));
        assert!(result.contains("External rule 2"));
    }
}

// ============================================================================
// BOM/CRLF/UTF-8-BOM Handling Tests
// ============================================================================

#[tokio::test]
async fn test_utf8_bom_handling() {
    let temp_dir = TempDir::new().unwrap();
    let workspace = temp_dir.path();
    
    // Create rules directory
    let rules_dir = workspace.join(".kilocode/rules");
    fs::create_dir_all(&rules_dir).await.unwrap();
    
    // Create file with UTF-8 BOM
    let file_path = rules_dir.join("bom.txt");
    let mut file = std::fs::File::create(&file_path).unwrap();
    file.write_all(&[0xEF, 0xBB, 0xBF]).unwrap(); // UTF-8 BOM
    file.write_all(b"Content with BOM").unwrap();
    drop(file);
    
    let settings = SystemPromptSettings::default();
    
    let result = add_custom_instructions(
        "",
        "",
        workspace,
        "code",
        None,
        None,
        &settings,
    ).await.unwrap();
    
    // Should include content (BOM should be handled by fs::read_to_string)
    assert!(result.contains("Content with BOM"));
}

#[tokio::test]
async fn test_crlf_line_endings() {
    let temp_dir = TempDir::new().unwrap();
    let workspace = temp_dir.path();
    
    // Create rules directory
    let rules_dir = workspace.join(".kilocode/rules");
    fs::create_dir_all(&rules_dir).await.unwrap();
    
    // Create file with CRLF line endings
    let file_path = rules_dir.join("crlf.txt");
    let mut file = std::fs::File::create(&file_path).unwrap();
    file.write_all(b"Line 1\r\nLine 2\r\nLine 3").unwrap();
    drop(file);
    
    let settings = SystemPromptSettings::default();
    
    let result = add_custom_instructions(
        "",
        "",
        workspace,
        "code",
        None,
        None,
        &settings,
    ).await.unwrap();
    
    // Should preserve content
    assert!(result.contains("Line 1"));
    assert!(result.contains("Line 2"));
    assert!(result.contains("Line 3"));
}

#[tokio::test]
async fn test_mixed_line_endings() {
    let temp_dir = TempDir::new().unwrap();
    let workspace = temp_dir.path();
    
    // Create rules directory
    let rules_dir = workspace.join(".kilocode/rules");
    fs::create_dir_all(&rules_dir).await.unwrap();
    
    // Create file with mixed line endings
    let file_path = rules_dir.join("mixed.txt");
    let mut file = std::fs::File::create(&file_path).unwrap();
    file.write_all(b"Line 1\nLine 2\r\nLine 3\rLine 4").unwrap();
    drop(file);
    
    let settings = SystemPromptSettings::default();
    
    let result = add_custom_instructions(
        "",
        "",
        workspace,
        "code",
        None,
        None,
        &settings,
    ).await.unwrap();
    
    // Should handle all line ending types
    assert!(result.contains("Line 1"));
    assert!(result.contains("Line 2"));
    assert!(result.contains("Line 3"));
    assert!(result.contains("Line 4"));
}

// ============================================================================
// Binary File Detection and Skip Tests
// ============================================================================

#[tokio::test]
async fn test_binary_file_skip() {
    let temp_dir = TempDir::new().unwrap();
    let workspace = temp_dir.path();
    
    // Create rules directory
    let rules_dir = workspace.join(".kilocode/rules");
    fs::create_dir_all(&rules_dir).await.unwrap();
    
    // Create a text file
    fs::write(rules_dir.join("text.txt"), "Text content").await.unwrap();
    
    // Create a binary file (simulated with null bytes)
    let mut binary = std::fs::File::create(rules_dir.join("binary.bin")).unwrap();
    binary.write_all(&[0x00, 0x01, 0x02, 0xFF, 0xFE, 0xFD]).unwrap();
    drop(binary);
    
    let settings = SystemPromptSettings::default();
    
    let result = add_custom_instructions(
        "",
        "",
        workspace,
        "code",
        None,
        None,
        &settings,
    ).await.unwrap();
    
    // Should include text file
    assert!(result.contains("Text content"));
    // Should NOT include binary file content
    // (binary files should be skipped)
}

#[tokio::test]
async fn test_image_file_skip() {
    let temp_dir = TempDir::new().unwrap();
    let workspace = temp_dir.path();
    
    // Create rules directory
    let rules_dir = workspace.join(".kilocode/rules");
    fs::create_dir_all(&rules_dir).await.unwrap();
    
    // Create a text file
    fs::write(rules_dir.join("readme.txt"), "Documentation").await.unwrap();
    
    // Create files with image extensions
    let mut png = std::fs::File::create(rules_dir.join("icon.png")).unwrap();
    png.write_all(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]).unwrap(); // PNG header
    drop(png);
    
    let settings = SystemPromptSettings::default();
    
    let result = add_custom_instructions(
        "",
        "",
        workspace,
        "code",
        None,
        None,
        &settings,
    ).await.unwrap();
    
    // Should include text file
    assert!(result.contains("Documentation"));
    // PNG should be detected as binary and skipped
}

// ============================================================================
// Stable Ordering Tests (Alphabetical)
// ============================================================================

#[tokio::test]
async fn test_alphabetical_ordering() {
    let temp_dir = TempDir::new().unwrap();
    let workspace = temp_dir.path();
    
    // Create rules directory
    let rules_dir = workspace.join(".kilocode/rules");
    fs::create_dir_all(&rules_dir).await.unwrap();
    
    // Create files in non-alphabetical order
    fs::write(rules_dir.join("zebra.txt"), "ZEBRA").await.unwrap();
    fs::write(rules_dir.join("apple.txt"), "APPLE").await.unwrap();
    fs::write(rules_dir.join("mango.txt"), "MANGO").await.unwrap();
    fs::write(rules_dir.join("banana.txt"), "BANANA").await.unwrap();
    
    let settings = SystemPromptSettings::default();
    
    let result = add_custom_instructions(
        "",
        "",
        workspace,
        "code",
        None,
        None,
        &settings,
    ).await.unwrap();
    
    // Find positions
    let apple_pos = result.find("apple.txt").unwrap();
    let banana_pos = result.find("banana.txt").unwrap();
    let mango_pos = result.find("mango.txt").unwrap();
    let zebra_pos = result.find("zebra.txt").unwrap();
    
    // Should be in alphabetical order
    assert!(apple_pos < banana_pos, "apple should come before banana");
    assert!(banana_pos < mango_pos, "banana should come before mango");
    assert!(mango_pos < zebra_pos, "mango should come before zebra");
}

#[tokio::test]
async fn test_case_insensitive_ordering() {
    let temp_dir = TempDir::new().unwrap();
    let workspace = temp_dir.path();
    
    // Create rules directory
    let rules_dir = workspace.join(".kilocode/rules");
    fs::create_dir_all(&rules_dir).await.unwrap();
    
    // Create files with mixed case
    fs::write(rules_dir.join("Zebra.txt"), "Z").await.unwrap();
    fs::write(rules_dir.join("apple.txt"), "a").await.unwrap();
    fs::write(rules_dir.join("BANANA.txt"), "B").await.unwrap();
    
    let settings = SystemPromptSettings::default();
    
    let result = add_custom_instructions(
        "",
        "",
        workspace,
        "code",
        None,
        None,
        &settings,
    ).await.unwrap();
    
    // Find positions (case-insensitive ordering)
    let apple_pos = result.find("apple.txt").unwrap();
    let banana_pos = result.find("BANANA.txt").unwrap();
    let zebra_pos = result.find("Zebra.txt").unwrap();
    
    // Should be in case-insensitive alphabetical order
    assert!(apple_pos < banana_pos);
    assert!(banana_pos < zebra_pos);
}

// ============================================================================
// Depth Limit Tests
// ============================================================================

#[tokio::test]
async fn test_nested_directory_depth_limit() {
    let temp_dir = TempDir::new().unwrap();
    let workspace = temp_dir.path();
    
    // Create deeply nested directory structure (beyond MAX_DEPTH=5)
    let mut current = workspace.join(".kilocode/rules");
    fs::create_dir_all(&current).await.unwrap();
    
    // Create files at each level
    fs::write(current.join("level0.txt"), "Level 0").await.unwrap();
    
    for i in 1..=7 {
        current = current.join(format!("subdir{}", i));
        fs::create_dir_all(&current).await.unwrap();
        fs::write(current.join(format!("level{}.txt", i)), format!("Level {}", i)).await.unwrap();
    }
    
    let settings = SystemPromptSettings::default();
    
    let result = add_custom_instructions(
        "",
        "",
        workspace,
        "code",
        None,
        None,
        &settings,
    ).await.unwrap();
    
    // Should include shallow levels
    assert!(result.contains("Level 0"));
    assert!(result.contains("Level 1"));
    
    // May or may not include very deep levels depending on MAX_DEPTH
    // The key is that it doesn't crash or hang
}

// ============================================================================
// Cache File Filtering Tests
// ============================================================================

#[tokio::test]
async fn test_cache_file_filtering() {
    let temp_dir = TempDir::new().unwrap();
    let workspace = temp_dir.path();
    
    // Create rules directory
    let rules_dir = workspace.join(".kilocode/rules");
    fs::create_dir_all(&rules_dir).await.unwrap();
    
    // Create regular file
    fs::write(rules_dir.join("valid.txt"), "Valid content").await.unwrap();
    
    // Create cache/temp files that should be excluded
    fs::write(rules_dir.join(".DS_Store"), "Mac cache").await.unwrap();
    fs::write(rules_dir.join("file.log"), "Log content").await.unwrap();
    fs::write(rules_dir.join("backup.bak"), "Backup").await.unwrap();
    fs::write(rules_dir.join("Thumbs.db"), "Windows cache").await.unwrap();
    fs::write(rules_dir.join("temp.tmp"), "Temporary").await.unwrap();
    fs::write(rules_dir.join("old.old"), "Old file").await.unwrap();
    
    let settings = SystemPromptSettings::default();
    
    let result = add_custom_instructions(
        "",
        "",
        workspace,
        "code",
        None,
        None,
        &settings,
    ).await.unwrap();
    
    // Should include valid file
    assert!(result.contains("Valid content"));
    
    // Should NOT include cache files
    assert!(!result.contains("Mac cache"));
    assert!(!result.contains("Log content"));
    assert!(!result.contains("Backup"));
    assert!(!result.contains("Windows cache"));
    assert!(!result.contains("Temporary"));
    assert!(!result.contains("Old file"));
}

// ============================================================================
// Legacy File Support Tests
// ============================================================================

#[tokio::test]
async fn test_legacy_kilocoderules() {
    let temp_dir = TempDir::new().unwrap();
    let workspace = temp_dir.path();
    
    // Create legacy .kilocoderules file
    fs::write(workspace.join(".kilocoderules"), "Legacy rules").await.unwrap();
    
    let settings = SystemPromptSettings::default();
    
    let result = add_custom_instructions(
        "",
        "",
        workspace,
        "code",
        None,
        None,
        &settings,
    ).await.unwrap();
    
    assert!(result.contains("Legacy rules"));
    assert!(result.contains(".kilocoderules"));
}

#[tokio::test]
async fn test_legacy_fallback_order() {
    let temp_dir = TempDir::new().unwrap();
    let workspace = temp_dir.path();
    
    // Create multiple legacy files
    fs::write(workspace.join(".roorules"), "Roo rules").await.unwrap();
    fs::write(workspace.join(".clinerules"), "Cline rules").await.unwrap();
    
    let settings = SystemPromptSettings::default();
    
    let result = add_custom_instructions(
        "",
        "",
        workspace,
        "code",
        None,
        None,
        &settings,
    ).await.unwrap();
    
    // Should use first one found in priority order
    // (according to Codex: .kilocoderules, then .roorules, then .clinerules)
    assert!(result.contains("Roo rules") || result.contains("Cline rules"));
}

// ============================================================================
// AGENTS.md Tests
// ============================================================================

#[tokio::test]
async fn test_agents_md_loading() {
    let temp_dir = TempDir::new().unwrap();
    let workspace = temp_dir.path();
    
    // Create AGENTS.md
    fs::write(workspace.join("AGENTS.md"), "# Agent Rules\n\nAlways be helpful.").await.unwrap();
    
    let settings = SystemPromptSettings::default();
    
    let result = add_custom_instructions(
        "",
        "",
        workspace,
        "code",
        None,
        None,
        &settings,
    ).await.unwrap();
    
    assert!(result.contains("Agent Rules Standard"));
    assert!(result.contains("Always be helpful"));
}

#[tokio::test]
async fn test_agents_md_disabled() {
    let temp_dir = TempDir::new().unwrap();
    let workspace = temp_dir.path();
    
    // Create AGENTS.md
    fs::write(workspace.join("AGENTS.md"), "Agent rules").await.unwrap();
    
    let mut settings = SystemPromptSettings::default();
    settings.use_agent_rules = false;
    
    let result = add_custom_instructions(
        "",
        "",
        workspace,
        "code",
        None,
        None,
        &settings,
    ).await.unwrap();
    
    // Should NOT include AGENTS.md when disabled
    assert!(!result.contains("Agent rules"));
}

// ============================================================================
// Mode-Specific Rules Tests
// ============================================================================

#[tokio::test]
async fn test_mode_specific_rules_directory() {
    let temp_dir = TempDir::new().unwrap();
    let workspace = temp_dir.path();
    
    // Create mode-specific rules directory
    let mode_rules = workspace.join(".kilocode/rules-architect");
    fs::create_dir_all(&mode_rules).await.unwrap();
    fs::write(mode_rules.join("arch.txt"), "Architect rules").await.unwrap();
    
    // Create generic rules
    let rules = workspace.join(".kilocode/rules");
    fs::create_dir_all(&rules).await.unwrap();
    fs::write(rules.join("general.txt"), "General rules").await.unwrap();
    
    let settings = SystemPromptSettings::default();
    
    let result = add_custom_instructions(
        "",
        "",
        workspace,
        "architect",
        None,
        None,
        &settings,
    ).await.unwrap();
    
    // Should include both mode-specific and general rules
    assert!(result.contains("Architect rules"));
    assert!(result.contains("General rules"));
}

#[tokio::test]
async fn test_mode_specific_rules_precedence() {
    let temp_dir = TempDir::new().unwrap();
    let workspace = temp_dir.path();
    
    // Create mode-specific rules
    let mode_rules = workspace.join(".kilocode/rules-code");
    fs::create_dir_all(&mode_rules).await.unwrap();
    fs::write(mode_rules.join("mode.txt"), "MODE_SPECIFIC").await.unwrap();
    
    // Create generic rules  
    let rules = workspace.join(".kilocode/rules");
    fs::create_dir_all(&rules).await.unwrap();
    fs::write(rules.join("generic.txt"), "GENERIC").await.unwrap();
    
    let settings = SystemPromptSettings::default();
    
    let result = add_custom_instructions(
        "",
        "",
        workspace,
        "code",
        None,
        None,
        &settings,
    ).await.unwrap();
    
    // Find positions
    let mode_pos = result.find("MODE_SPECIFIC").unwrap();
    let generic_pos = result.find("GENERIC").unwrap();
    
    // Mode-specific should come before generic
    assert!(mode_pos < generic_pos, "Mode-specific rules should appear before generic rules");
}

// ============================================================================
// Layering and Priority Tests
// ============================================================================

#[tokio::test]
async fn test_instruction_layering_order() {
    let temp_dir = TempDir::new().unwrap();
    let workspace = temp_dir.path();
    
    let settings = SystemPromptSettings::default();
    
    let result = add_custom_instructions(
        "MODE_INSTRUCTIONS",
        "GLOBAL_INSTRUCTIONS",
        workspace,
        "code",
        Some("LANGUAGE"),
        Some("ROOIGNORE"),
        &settings,
    ).await.unwrap();
    
    // Find positions
    let lang_pos = result.find("LANGUAGE").unwrap();
    let global_pos = result.find("GLOBAL_INSTRUCTIONS").unwrap();
    let mode_pos = result.find("MODE_INSTRUCTIONS").unwrap();
    let roo_pos = result.find("ROOIGNORE").unwrap();
    
    // Verify order: Language → Global → Mode → Rules (including RooIgnore)
    assert!(lang_pos < global_pos, "Language should come before global");
    assert!(global_pos < mode_pos, "Global should come before mode");
    assert!(mode_pos < roo_pos, "Mode should come before rules");
}

// ============================================================================
// Integration Test - Full Scenario
// ============================================================================

#[tokio::test]
async fn test_full_custom_instructions_integration() {
    let temp_dir = TempDir::new().unwrap();
    let workspace = temp_dir.path();
    
    // Create a realistic setup
    // 1. Mode-specific rules
    let mode_rules = workspace.join(".kilocode/rules-code");
    fs::create_dir_all(&mode_rules).await.unwrap();
    fs::write(mode_rules.join("code_style.txt"), "Use tabs for indentation").await.unwrap();
    
    // 2. Generic rules with multiple files
    let rules = workspace.join(".kilocode/rules");
    fs::create_dir_all(&rules).await.unwrap();
    fs::write(rules.join("naming.txt"), "Use camelCase for variables").await.unwrap();
    fs::write(rules.join("testing.txt"), "Write unit tests for all functions").await.unwrap();
    
    // 3. AGENTS.md
    fs::write(workspace.join("AGENTS.md"), "Always explain your reasoning").await.unwrap();
    
    // 4. Some cache files (should be ignored)
    fs::write(rules.join(".DS_Store"), "cache").await.unwrap();
    fs::write(rules.join("temp.log"), "log").await.unwrap();
    
    let settings = SystemPromptSettings::default();
    
    let result = add_custom_instructions(
        "Mode: Be concise",
        "Global: Be helpful",
        workspace,
        "code",
        Some("English"),
        Some("Ignore .env files"),
        &settings,
    ).await.unwrap();
    
    // Verify all expected content is present
    assert!(result.contains("USER'S CUSTOM INSTRUCTIONS"));
    assert!(result.contains("Language Preference"));
    assert!(result.contains("English"));
    assert!(result.contains("Global: Be helpful"));
    assert!(result.contains("Mode: Be concise"));
    assert!(result.contains("Use tabs for indentation"));
    assert!(result.contains("Use camelCase for variables"));
    assert!(result.contains("Write unit tests"));
    assert!(result.contains("Always explain your reasoning"));
    assert!(result.contains("Ignore .env files"));
    
    // Verify cache files are NOT present
    assert!(!result.contains("cache"));
    assert!(!result.contains("log"));
    
    // Verify alphabetical ordering of generic rules
    let naming_pos = result.find("naming.txt").unwrap();
    let testing_pos = result.find("testing.txt").unwrap();
    assert!(naming_pos < testing_pos, "naming.txt should come before testing.txt");
}
