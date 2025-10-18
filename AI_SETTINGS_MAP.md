# Codex → Lapce AI Settings Mapping (Complete Reference)

**Purpose**: Map every Codex (VS Code extension) setting to its Lapce AI equivalent.  
**Sources**: 
- `Codex/src/package.json` (contributes.configuration)
- `Codex/webview-ui/src/context/ExtensionStateContext.tsx` (runtime state + defaults)
- `Codex/webview-ui/src/components/settings/SettingsView.tsx` (UI postMessage types)

**Target**: `lapce-app/src/config/ai.rs` (AIConfig struct)

---

## Critical Priority (10 settings)

### 1. Model Selector & Defaults
| Codex Key | Codex Default | Lapce Key | Rust Type | UI Location |
|-----------|---------------|-----------|-----------|-------------|
| `kilocodeDefaultModel` | `openRouterDefaultModelId` | `default_model` | `String` | AI → General |
| N/A (UI-driven) | N/A | `show_model_selector` | `bool` | AI → General |

### 2. API Configuration
| Codex Key | Codex Default | Lapce Key | Rust Type | UI Location |
|-----------|---------------|-----------|-----------|-------------|
| `kilo-code.apiRequestTimeout` | `600` | `api_request_timeout_secs` | `u32` | AI → General |

### 3. Uploads & Attachments
| Codex Key | Codex Default | Lapce Key | Rust Type | UI Location |
|-----------|---------------|-----------|-----------|-------------|
| `maxImageFileSize` | `5` | `max_image_file_mb` | `u32` | AI → Uploads |
| `maxTotalImageSize` | `20` | `max_total_image_mb` | `u32` | AI → Uploads |
| `maxReadFileLine` | `-1` | `max_read_file_lines` | `i32` | AI → Uploads |

### 4. History & Display
| Codex Key | Codex Default | Lapce Key | Rust Type | UI Location |
|-----------|---------------|-----------|-----------|-------------|
| `historyPreviewCollapsed` | `false` | `history_preview_collapsed` | `bool` | AI → History |
| `includeTaskHistoryInEnhance` | `true` | `include_task_history_in_enhance` | `bool` | AI → History |
| `showTaskTimeline` | `true` | `show_task_timeline` | `bool` | AI → Display |
| `showTimestamps` | `true` | `show_timestamps` | `bool` | AI → Display |

---

## High Priority (15 settings)

### 5. Modes
| Codex Key | Codex Default | Lapce Key | Rust Type | UI Location |
|-----------|---------------|-----------|-----------|-------------|
| `mode` | `defaultModeSlug` | `mode` | `String` | AI → Modes |
| `customModes` | `[]` | `custom_modes` | `Vec<ModeConfig>` | AI → Modes |
| `customModePrompts` | `defaultPrompts` | `custom_mode_prompts` | `serde_json::Value` | AI → Modes |
| `customSupportPrompts` | `{}` | `custom_support_prompts` | `serde_json::Value` | AI → Modes |
| `hasOpenedModeSelector` | `false` | `has_opened_mode_selector` | `bool` | AI → Modes |
| `fastApplyModel` | `""` | `fast_apply_model` | `String` | AI → Modes |

### 6. Auto-Approve (Core Toggles)
| Codex Key | Codex Default | Lapce Key | Rust Type | UI Location |
|-----------|---------------|-----------|-----------|-------------|
| `alwaysAllowReadOnly` | `true` | `always_allow_read_only` | `bool` | AI → Auto-Approve |
| `alwaysAllowReadOnlyOutsideWorkspace` | `false` | `always_allow_read_only_outside_workspace` | `bool` | AI → Auto-Approve |
| `alwaysAllowWrite` | `true` | `always_allow_write` | `bool` | AI → Auto-Approve |
| `alwaysAllowWriteOutsideWorkspace` | `false` | `always_allow_write_outside_workspace` | `bool` | AI → Auto-Approve |
| `alwaysAllowWriteProtected` | `false` | `always_allow_write_protected` | `bool` | AI → Auto-Approve |
| `alwaysAllowBrowser` | `false` | `always_allow_browser` | `bool` | AI → Auto-Approve |
| `alwaysAllowExecute` | `false` | `always_allow_execute` | `bool` | AI → Auto-Approve |
| `alwaysAllowMcp` | `false` | `always_allow_mcp` | `bool` | AI → Auto-Approve |
| `alwaysAllowModeSwitch` | `false` | `always_allow_mode_switch` | `bool` | AI → Auto-Approve |
| `alwaysAllowSubtasks` | `false` | `always_allow_subtasks` | `bool` | AI → Auto-Approve |
| `alwaysApproveResubmit` | `false` | `always_approve_resubmit` | `bool` | AI → Auto-Approve |
| `alwaysAllowFollowupQuestions` | `false` | `always_allow_followup_questions` | `bool` | AI → Auto-Approve |
| `alwaysAllowUpdateTodoList` | `true` | `always_allow_update_todo_list` | `bool` | AI → Auto-Approve |

### 7. Auto-Approve (Limits & Lists)
| Codex Key | Codex Default | Lapce Key | Rust Type | UI Location |
|-----------|---------------|-----------|-----------|-------------|
| `allowedCommands` | `[]` | `allowed_commands` | `Vec<String>` | AI → Auto-Approve |
| `kilo-code.deniedCommands` | `[]` | `denied_commands` | `Vec<String>` | AI → Auto-Approve |
| `allowedMaxRequests` | `undefined` | `allowed_max_requests` | `Option<u32>` | AI → Auto-Approve |
| `allowedMaxCost` | `undefined` | `allowed_max_cost` | `Option<f32>` | AI → Auto-Approve |
| `requestDelaySeconds` | `5` | `request_delay_seconds` | `u32` | AI → Auto-Approve |
| `followupAutoApproveTimeoutMs` | `undefined` | `followup_auto_approve_timeout_ms` | `Option<u32>` | AI → Auto-Approve |
| `showAutoApproveMenu` | `false` | `show_auto_approve_menu` | `bool` | AI → Auto-Approve |

### 8. Task Management
| Codex Key | Codex Default | Lapce Key | Rust Type | UI Location |
|-----------|---------------|-----------|-----------|-------------|
| `kilo-code.newTaskRequireTodos` | `false` | `new_task_require_todos` | `bool` | AI → Task Management |
| `maxOpenTabsContext` | `20` | `max_open_tabs_context` | `u32` | AI → Task Management |
| `maxWorkspaceFiles` | `200` | `max_workspace_files` | `u32` | AI → Task Management |
| `kilo-code.useAgentRules` | `true` | `use_agent_rules` | `bool` | AI → Task Management |

---

## Medium Priority (33 settings)

### 9. Providers & API Config
| Codex Key | Codex Default | Lapce Key | Rust Type | UI Location |
|-----------|---------------|-----------|-----------|-------------|
| `currentApiConfigName` | `"default"` | `current_api_config_name` | `String` | AI → Providers |
| `pinnedApiConfigs` | `{}` | `pinned_api_configs` | `HashMap<String, bool>` | AI → Providers |
| `condensingApiConfigId` | `""` | `condensing_api_config_id` | `String` | AI → Providers |
| `enhancementApiConfigId` | `""` | `enhancement_api_config_id` | `String` | AI → Providers |
| `commitMessageApiConfigId` | `""` | `commit_message_api_config_id` | `String` | AI → Providers |
| `apiConfiguration` | `{}` | `api_configuration` | `serde_json::Value` | AI → Providers |

### 10. Browser
| Codex Key | Codex Default | Lapce Key | Rust Type | UI Location |
|-----------|---------------|-----------|-----------|-------------|
| `browserToolEnabled` | `true` | `browser_tool_enabled` | `bool` | AI → Browser |
| `browserViewportSize` | `"900x600"` | `browser_viewport_size` | `String` | AI → Browser |
| `screenshotQuality` | `75` | `screenshot_quality` | `u8` | AI → Browser |
| `remoteBrowserHost` | `""` | `remote_browser_host` | `String` | AI → Browser |
| `remoteBrowserEnabled` | `false` | `remote_browser_enabled` | `bool` | AI → Browser |

### 11. Terminal
| Codex Key | Codex Default | Lapce Key | Rust Type | UI Location |
|-----------|---------------|-----------|-----------|-------------|
| `terminalOutputLineLimit` | `500` | `terminal_output_line_limit` | `u32` | AI → Terminal |
| `terminalOutputCharacterLimit` | `50000` | `terminal_output_character_limit` | `u32` | AI → Terminal |
| `terminalShellIntegrationTimeout` | `4000` | `terminal_shell_integration_timeout` | `u32` | AI → Terminal |
| `terminalShellIntegrationDisabled` | `false` | `terminal_shell_integration_disabled` | `bool` | AI → Terminal |
| `terminalZdotdir` | `false` | `terminal_zdotdir` | `bool` | AI → Terminal |
| `terminalZshOhMy` | `false` | `terminal_zsh_oh_my` | `bool` | AI → Terminal |
| `terminalZshP10k` | `false` | `terminal_zsh_p10k` | `bool` | AI → Terminal |
| `terminalCompressProgressBar` | `true` | `terminal_compress_progress_bar` | `bool` | AI → Terminal |
| `terminalCommandDelay` | `0` | `terminal_command_delay` | `u32` | AI → Terminal |
| `terminalPowershellCounter` | `false` | `terminal_powershell_counter` | `bool` | AI → Terminal |
| `terminalZshClearEolMark` | `false` | `terminal_zsh_clear_eol_mark` | `bool` | AI → Terminal |
| `terminalCommandApiConfigId` | `""` | `terminal_command_api_config_id` | `String` | AI → Terminal |

### 12. Display
| Codex Key | Codex Default | Lapce Key | Rust Type | UI Location |
|-----------|---------------|-----------|-----------|-------------|
| `reasoningBlockCollapsed` | `true` | `reasoning_block_collapsed` | `bool` | AI → Display |
| `hideCostBelowThreshold` | `0.0` | `hide_cost_below_threshold` | `f32` | AI → Display |

### 13. Notifications
| Codex Key | Codex Default | Lapce Key | Rust Type | UI Location |
|-----------|---------------|-----------|-----------|-------------|
| `soundEnabled` | `false` | `sound_enabled` | `bool` | AI → Notifications |
| `soundVolume` | `0.5` | `sound_volume` | `f32` | AI → Notifications |
| `ttsEnabled` | `false` | `tts_enabled` | `bool` | AI → Notifications |
| `ttsSpeed` | `1.0` | `tts_speed` | `f32` | AI → Notifications |
| `systemNotificationsEnabled` | `false` | `system_notifications_enabled` | `bool` | AI → Notifications |

### 14. Context Management
| Codex Key | Codex Default | Lapce Key | Rust Type | UI Location |
|-----------|---------------|-----------|-----------|-------------|
| `autoCondenseContext` | `true` | `auto_condense_context` | `bool` | AI → Context |
| `autoCondenseContextPercent` | `100` | `auto_condense_context_percent` | `u8` | AI → Context |
| `writeDelayMs` | `1000` | `write_delay_ms` | `u32` | AI → Context |
| `fuzzyMatchThreshold` | `1.0` | `fuzzy_match_threshold` | `f32` | AI → Context |

### 15. Performance
| Codex Key | Codex Default | Lapce Key | Rust Type | UI Location |
|-----------|---------------|-----------|-----------|-------------|
| `maxConcurrentFileReads` | `5` | `max_concurrent_file_reads` | `u32` | AI → Performance |
| `allowVeryLargeReads` | `false` | `allow_very_large_reads` | `bool` | AI → Performance |

### 16. Diagnostics
| Codex Key | Codex Default | Lapce Key | Rust Type | UI Location |
|-----------|---------------|-----------|-----------|-------------|
| `includeDiagnosticMessages` | `true` | `include_diagnostic_messages` | `bool` | AI → Diagnostics |
| `maxDiagnosticMessages` | `50` | `max_diagnostic_messages` | `u32` | AI → Diagnostics |

### 17. Image Generation
| Codex Key | Codex Default | Lapce Key | Rust Type | UI Location |
|-----------|---------------|-----------|-----------|-------------|
| `openRouterImageApiKey` | `""` | `openrouter_image_api_key` | `String` | AI → Image Gen |
| `kiloCodeImageApiKey` | `""` | `kilocode_image_api_key` | `String` | AI → Image Gen |
| `openRouterImageGenerationSelectedModel` | `""` | `openrouter_image_generation_model` | `String` | AI → Image Gen |

### 18. MCP
| Codex Key | Codex Default | Lapce Key | Rust Type | UI Location |
|-----------|---------------|-----------|-----------|-------------|
| `mcpEnabled` | `true` | `mcp_enabled` | `bool` | AI → MCP |
| `enableMcpServerCreation` | `false` | `enable_mcp_server_creation` | `bool` | AI → MCP |

---

## Low Priority (15 settings)

### 19. Cloud
| Codex Key | Codex Default | Lapce Key | Rust Type | UI Location |
|-----------|---------------|-----------|-----------|-------------|
| `cloudIsAuthenticated` | `false` | `cloud_is_authenticated` | `bool` | AI → Cloud (read-only) |
| `cloudOrganizations` | `[]` | `cloud_organizations` | `Vec<serde_json::Value>` | AI → Cloud (read-only) |
| `sharingEnabled` | `false` | `sharing_enabled` | `bool` | AI → Cloud |
| `organizationAllowList` | `ORGANIZATION_ALLOW_ALL` | `organization_allow_list` | `String` | AI → Cloud |
| `organizationSettingsVersion` | `-1` | `organization_settings_version` | `i32` | AI → Cloud |

### 20. Marketplace
| Codex Key | Codex Default | Lapce Key | Rust Type | UI Location |
|-----------|---------------|-----------|-----------|-------------|
| `marketplaceItems` | `[]` | `marketplace_items` | `Vec<serde_json::Value>` | AI → Marketplace (read-only) |
| `marketplaceInstalledMetadata` | `{}` | `marketplace_installed_metadata` | `serde_json::Value` | AI → Marketplace (read-only) |

### 21. Misc Config
| Codex Key | Codex Default | Lapce Key | Rust Type | UI Location |
|-----------|---------------|-----------|-----------|-------------|
| `language` | `"en"` | `language` | `String` | AI → Language |
| `kilo-code.autoImportSettingsPath` | `""` | `auto_import_settings_path` | `String` | AI → Advanced |
| `kilo-code.customStoragePath` | `""` | `custom_storage_path` | `String` | AI → Advanced |
| `kilo-code.enableCodeActions` | `true` | `enable_code_actions` | `bool` | AI → Advanced |
| `kilo-code.preventCompletionWithOpenTodos` | `false` | `prevent_completion_with_open_todos` | `bool` | AI → Advanced |
| `kilo-code.commandExecutionTimeout` | `0` | `command_execution_timeout_secs` | `u32` | AI → Advanced |
| `kilo-code.commandTimeoutAllowlist` | `[]` | `command_timeout_allowlist` | `Vec<String>` | AI → Advanced |
| `kilo-code.codeIndex.embeddingBatchSize` | `60` | `code_index_embedding_batch_size` | `u32` | AI → Advanced |

---

## Special UI-Only Settings (not persisted in Codex backend; UI state only)

| Codex Key | Codex Default | Lapce Key | Rust Type | Notes |
|-----------|---------------|-----------|-----------|-------|
| `autoApprovalEnabled` | `true` | `auto_approval_enabled` | `bool` | Master toggle (runtime; persists) |
| `diffEnabled` | `false` | `diff_enabled` | `bool` | Diff UI toggle |
| `enableCheckpoints` | `true` | `enable_checkpoints` | `bool` | Checkpoint toggle |
| `customCondensingPrompt` | `""` | `custom_condensing_prompt` | `String` | User-provided prompt override |
| `profileThresholds` | `{}` | `profile_thresholds` | `HashMap<String, f32>` | Cost profile thresholds |

---

## Summary Statistics

- **Total Codex Settings Mapped**: 110+
- **Critical (must-have for Phase 1)**: 10
- **High (auto-approve + modes)**: 15
- **Medium (panels)**: 33
- **Low (cloud/marketplace/misc)**: 15
- **UI-only (runtime state)**: 5

---

## Implementation Notes

1. **Naming Convention**: Codex uses camelCase; Lapce uses snake_case.
2. **Defaults**: All defaults sourced from `ExtensionStateContext.tsx` line 215–310.
3. **Types**: Rust types chosen for safety (Option for nullable, Vec for arrays, HashMap for objects).
4. **Validation**: Number ranges to be enforced in UI (e.g., screenshot_quality 0–100).
5. **Secrets**: API keys stored as strings in config but will be secured via engine keyring later (IPC phase).
6. **Read-only**: Cloud/marketplace data is read-only in UI until IPC wiring complete.

---

## File Locations

- **Lapce Config**: `lapce-app/src/config/ai.rs`
- **Lapce Settings UI**: `lapce-app/src/settings.rs`
- **Codex Reference**:
  - `Codex/src/package.json` (lines 517–613)
  - `Codex/webview-ui/src/context/ExtensionStateContext.tsx` (lines 32–190, 215–310)
  - `Codex/webview-ui/src/components/settings/SettingsView.tsx` (lines 396–487)

---

**Status**: ✅ Complete mapping reference  
**Next**: Implement AIConfig struct in `lapce-app/src/config/ai.rs`
