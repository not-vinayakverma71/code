/// LSP Spec Compliance (LSP-034)
/// Validate lsp-types 0.95 mappings, URI/Path conversions, optional fields

use std::path::{Path, PathBuf};
use anyhow::{Result, anyhow};
use lsp_types::{Url, Position, Range, Location};

/// URI/Path conversion utilities
pub struct UriPathConverter;

impl UriPathConverter {
    /// Convert file path to URI
    pub fn path_to_uri(path: &Path) -> Result<Url> {
        // Ensure absolute path
        let absolute = if path.is_absolute() {
            path.to_path_buf()
        } else {
            std::env::current_dir()?.join(path)
        };
        
        // Convert to URI
        Url::from_file_path(&absolute)
            .map_err(|_| anyhow!("Failed to convert path to URI: {:?}", path))
    }
    
    /// Convert URI to file path
    pub fn uri_to_path(uri: &Url) -> Result<PathBuf> {
        if uri.scheme() != "file" {
            return Err(anyhow!("URI scheme must be 'file', got: {}", uri.scheme()));
        }
        
        uri.to_file_path()
            .map_err(|_| anyhow!("Failed to convert URI to path: {}", uri))
    }
    
    /// Validate URI format
    pub fn validate_uri(uri: &str) -> Result<Url> {
        let parsed = Url::parse(uri)
            .map_err(|e| anyhow!("Invalid URI '{}': {}", uri, e))?;
        
        if parsed.scheme() != "file" {
            return Err(anyhow!("Only file:// URIs are supported, got: {}", parsed.scheme()));
        }
        
        Ok(parsed)
    }
    
    /// Normalize URI (handle Windows paths, trailing slashes, etc.)
    pub fn normalize_uri(uri: &Url) -> Result<Url> {
        let path = Self::uri_to_path(uri)?;
        Self::path_to_uri(&path)
    }
}

/// Position validation and utilities
pub struct PositionValidator;

impl PositionValidator {
    /// Validate position is within document bounds
    pub fn validate(position: &Position, line_count: u32, line_lengths: &[u32]) -> Result<()> {
        if position.line >= line_count {
            return Err(anyhow!(
                "Position line {} exceeds document line count {}",
                position.line,
                line_count
            ));
        }
        
        let line_index = position.line as usize;
        if line_index < line_lengths.len() {
            let line_length = line_lengths[line_index];
            if position.character > line_length {
                return Err(anyhow!(
                    "Position character {} exceeds line {} length {}",
                    position.character,
                    position.line,
                    line_length
                ));
            }
        }
        
        Ok(())
    }
    
    /// Clamp position to document bounds
    pub fn clamp(position: Position, line_count: u32, line_lengths: &[u32]) -> Position {
        let line = position.line.min(line_count.saturating_sub(1));
        let line_index = line as usize;
        
        let character = if line_index < line_lengths.len() {
            position.character.min(line_lengths[line_index])
        } else {
            0
        };
        
        Position { line, character }
    }
    
    /// Check if position is at document end
    pub fn is_end_of_document(position: &Position, line_count: u32, line_lengths: &[u32]) -> bool {
        if position.line + 1 == line_count {
            let line_index = position.line as usize;
            if line_index < line_lengths.len() {
                return position.character >= line_lengths[line_index];
            }
        }
        false
    }
}

/// Range validation and utilities
pub struct RangeValidator;

impl RangeValidator {
    /// Validate range
    pub fn validate(range: &Range) -> Result<()> {
        // Check start is before or equal to end
        if range.start.line > range.end.line {
            return Err(anyhow!(
                "Range start line {} is after end line {}",
                range.start.line,
                range.end.line
            ));
        }
        
        if range.start.line == range.end.line && range.start.character > range.end.character {
            return Err(anyhow!(
                "Range start character {} is after end character {} on line {}",
                range.start.character,
                range.end.character,
                range.start.line
            ));
        }
        
        Ok(())
    }
    
    /// Check if range is empty
    pub fn is_empty(range: &Range) -> bool {
        range.start == range.end
    }
    
    /// Check if ranges overlap
    pub fn overlaps(a: &Range, b: &Range) -> bool {
        // Check if a ends before b starts
        if a.end.line < b.start.line {
            return false;
        }
        if a.end.line == b.start.line && a.end.character <= b.start.character {
            return false;
        }
        
        // Check if b ends before a starts
        if b.end.line < a.start.line {
            return false;
        }
        if b.end.line == a.start.line && b.end.character <= a.start.character {
            return false;
        }
        
        true
    }
    
    /// Merge overlapping ranges
    pub fn merge(ranges: &[Range]) -> Vec<Range> {
        if ranges.is_empty() {
            return Vec::new();
        }
        
        let mut sorted = ranges.to_vec();
        sorted.sort_by(|a, b| {
            a.start.line.cmp(&b.start.line)
                .then(a.start.character.cmp(&b.start.character))
        });
        
        let mut merged = vec![sorted[0]];
        
        for range in sorted.iter().skip(1) {
            let last = merged.last_mut().unwrap();
            
            if Self::overlaps(last, range) {
                // Merge ranges
                if range.end.line > last.end.line {
                    last.end = range.end;
                } else if range.end.line == last.end.line && range.end.character > last.end.character {
                    last.end = range.end;
                }
            } else {
                merged.push(*range);
            }
        }
        
        merged
    }
}

/// Location validation
pub struct LocationValidator;

impl LocationValidator {
    /// Validate location
    pub fn validate(location: &Location) -> Result<()> {
        // Validate URI
        UriPathConverter::validate_uri(location.uri.as_str())?;
        
        // Validate range
        RangeValidator::validate(&location.range)?;
        
        Ok(())
    }
}

/// Optional field helpers
pub struct OptionalFieldHelper;

impl OptionalFieldHelper {
    /// Check if optional string is empty or None
    pub fn is_empty_or_none(value: &Option<String>) -> bool {
        value.as_ref().map(|s| s.is_empty()).unwrap_or(true)
    }
    
    /// Get optional value or default
    pub fn get_or<T: Clone>(value: &Option<T>, default: T) -> T {
        value.clone().unwrap_or(default)
    }
    
    /// Map optional value
    pub fn map_optional<T, U, F>(value: Option<T>, f: F) -> Option<U>
    where
        F: FnOnce(T) -> U,
    {
        value.map(f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_path_to_uri() {
        let path = Path::new("/tmp/test.rs");
        let uri = UriPathConverter::path_to_uri(path).unwrap();
        assert_eq!(uri.scheme(), "file");
        assert!(uri.path().ends_with("/tmp/test.rs"));
    }
    
    #[test]
    fn test_uri_to_path() {
        let uri = Url::parse("file:///tmp/test.rs").unwrap();
        let path = UriPathConverter::uri_to_path(&uri).unwrap();
        assert_eq!(path, PathBuf::from("/tmp/test.rs"));
    }
    
    #[test]
    fn test_validate_uri() {
        assert!(UriPathConverter::validate_uri("file:///tmp/test.rs").is_ok());
        assert!(UriPathConverter::validate_uri("http://example.com").is_err());
        assert!(UriPathConverter::validate_uri("invalid").is_err());
    }
    
    #[test]
    fn test_position_validation() {
        let line_lengths = vec![10, 20, 5];
        
        // Valid positions
        assert!(PositionValidator::validate(
            &Position::new(0, 5),
            3,
            &line_lengths
        ).is_ok());
        
        assert!(PositionValidator::validate(
            &Position::new(1, 20),
            3,
            &line_lengths
        ).is_ok());
        
        // Invalid: line out of bounds
        assert!(PositionValidator::validate(
            &Position::new(3, 0),
            3,
            &line_lengths
        ).is_err());
        
        // Invalid: character out of bounds
        assert!(PositionValidator::validate(
            &Position::new(0, 11),
            3,
            &line_lengths
        ).is_err());
    }
    
    #[test]
    fn test_position_clamp() {
        let line_lengths = vec![10, 20, 5];
        
        // Clamp line
        let pos = PositionValidator::clamp(Position::new(5, 0), 3, &line_lengths);
        assert_eq!(pos.line, 2);
        
        // Clamp character
        let pos = PositionValidator::clamp(Position::new(0, 15), 3, &line_lengths);
        assert_eq!(pos.character, 10);
    }
    
    #[test]
    fn test_range_validation() {
        // Valid range
        let range = Range::new(Position::new(0, 0), Position::new(1, 10));
        assert!(RangeValidator::validate(&range).is_ok());
        
        // Invalid: start after end (line)
        let range = Range::new(Position::new(2, 0), Position::new(1, 10));
        assert!(RangeValidator::validate(&range).is_err());
        
        // Invalid: start after end (character)
        let range = Range::new(Position::new(0, 10), Position::new(0, 5));
        assert!(RangeValidator::validate(&range).is_err());
    }
    
    #[test]
    fn test_range_empty() {
        let range = Range::new(Position::new(1, 5), Position::new(1, 5));
        assert!(RangeValidator::is_empty(&range));
        
        let range = Range::new(Position::new(1, 5), Position::new(1, 10));
        assert!(!RangeValidator::is_empty(&range));
    }
    
    #[test]
    fn test_range_overlaps() {
        let a = Range::new(Position::new(0, 0), Position::new(1, 10));
        let b = Range::new(Position::new(0, 5), Position::new(2, 0));
        assert!(RangeValidator::overlaps(&a, &b));
        
        let a = Range::new(Position::new(0, 0), Position::new(1, 5));
        let b = Range::new(Position::new(2, 0), Position::new(3, 0));
        assert!(!RangeValidator::overlaps(&a, &b));
    }
    
    #[test]
    fn test_range_merge() {
        let ranges = vec![
            Range::new(Position::new(0, 0), Position::new(0, 10)),
            Range::new(Position::new(0, 5), Position::new(1, 5)),
            Range::new(Position::new(2, 0), Position::new(2, 10)),
        ];
        
        let merged = RangeValidator::merge(&ranges);
        assert_eq!(merged.len(), 2); // First two should merge
        assert_eq!(merged[0].start, Position::new(0, 0));
        assert_eq!(merged[0].end, Position::new(1, 5));
        assert_eq!(merged[1].start, Position::new(2, 0));
        assert_eq!(merged[1].end, Position::new(2, 10));
    }
    
    #[test]
    fn test_location_validation() {
        let location = Location::new(
            Url::parse("file:///tmp/test.rs").unwrap(),
            Range::new(Position::new(0, 0), Position::new(1, 10))
        );
        assert!(LocationValidator::validate(&location).is_ok());
        
        let location = Location::new(
            Url::parse("http://example.com").unwrap(),
            Range::new(Position::new(0, 0), Position::new(1, 10))
        );
        assert!(LocationValidator::validate(&location).is_err());
    }
}
