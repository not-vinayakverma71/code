// Utility functions for diff engine
use super::{DiffHunkV2, DiffLine, DiffMetadata};
use similar::Change;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineEnding {
    Lf,     // Unix: \n
    CrLf,   // Windows: \r\n
    Cr,     // Old Mac: \r
    Mixed,  // Mixed endings detected
    None,   // No line endings (single line)
}

pub fn detect_line_ending(content: &str) -> LineEnding {
    let has_crlf = content.contains("\r\n");
    let has_lf = content.contains('\n');
    let has_cr = content.contains('\r');
    
    match (has_crlf, has_lf, has_cr) {
        (true, true, _) => {
            if content.split("\r\n").any(|s| s.contains('\n')) {
                LineEnding::Mixed
            } else {
                LineEnding::CrLf
            }
        }
        (true, false, _) => LineEnding::CrLf,
        (false, true, _) => LineEnding::Lf,
        (false, false, true) => LineEnding::Cr,
        _ => LineEnding::None,
    }
}

pub fn calculate_checksum(content: &str) -> String {
    use sha2::{Sha256, Digest};
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("{:x}", hasher.finalize())
}

pub fn create_hunk_from_change(change: &Change<&str>, line_no: usize) -> DiffHunkV2 {
    let mut lines = Vec::new();
    
    match change.tag() {
        similar::ChangeTag::Delete => {
            lines.push(DiffLine::Deletion(change.value().trim_end().to_string()));
            DiffHunkV2 {
                original_start: line_no + 1,
                original_count: 1,
                modified_start: line_no + 1,
                modified_count: 0,
                lines,
                context_before: Vec::new(),
                context_after: Vec::new(),
            }
        }
        similar::ChangeTag::Insert => {
            lines.push(DiffLine::Addition(change.value().trim_end().to_string()));
            DiffHunkV2 {
                original_start: line_no + 1,
                original_count: 0,
                modified_start: line_no + 1,
                modified_count: 1,
                lines,
                context_before: Vec::new(),
                context_after: Vec::new(),
            }
        }
        similar::ChangeTag::Equal => {
            lines.push(DiffLine::Context(change.value().trim_end().to_string()));
            DiffHunkV2 {
                original_start: line_no + 1,
                original_count: 1,
                modified_start: line_no + 1,
                modified_count: 1,
                lines,
                context_before: Vec::new(),
                context_after: Vec::new(),
            }
        }
    }
}

pub fn add_change_to_hunk(hunk: &mut DiffHunkV2, change: &Change<&str>) {
    match change.tag() {
        similar::ChangeTag::Delete => {
            hunk.lines.push(DiffLine::Deletion(change.value().trim_end().to_string()));
            hunk.original_count += 1;
        }
        similar::ChangeTag::Insert => {
            hunk.lines.push(DiffLine::Addition(change.value().trim_end().to_string()));
            hunk.modified_count += 1;
        }
        similar::ChangeTag::Equal => {
            hunk.lines.push(DiffLine::Context(change.value().trim_end().to_string()));
            hunk.original_count += 1;
            hunk.modified_count += 1;
        }
    }
}

pub fn fuzzy_line_match(actual: &str, expected: &str) -> bool {
    // Normalize whitespace for comparison
    let actual_normalized = actual.trim().replace(|c: char| c.is_whitespace(), " ");
    let expected_normalized = expected.trim().replace(|c: char| c.is_whitespace(), " ");
    
    // Allow small differences
    if actual_normalized == expected_normalized {
        return true;
    }
    
    // Calculate similarity
    let similarity = strsim::normalized_levenshtein(&actual_normalized, &expected_normalized);
    similarity > 0.9 // 90% similarity threshold
}

pub fn restore_line_endings(content: &str, metadata: &DiffMetadata) -> String {
    let mut result = content.to_string();
    
    // Restore line endings
    if let Some(ref ending_str) = metadata.line_ending {
        let ending = match ending_str.as_str() {
            "CrLf" => "\r\n",
            "Cr" => "\r",
            _ => "\n",
        };
        
        if ending != "\n" {
            result = result.replace('\n', ending);
        }
    }
    
    // Restore trailing newline
    if metadata.has_trailing_newline && !result.ends_with('\n') && !result.ends_with("\r\n") {
        result.push('\n');
    }
    
    result
}
