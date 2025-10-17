//! Tests for prompt system
//!
//! P6: Loader tests (symlinks, BOM, CRLF, binary skip) ✅
//! P8: Section snapshot tests (exact string matching vs Codex) ✅
//! P14: Integration tests (end-to-end prompt builds) ✅

#[cfg(test)]
mod loader_tests;

#[cfg(test)]
mod section_snapshot_tests;

#[cfg(test)]
mod integration_tests;
