// File system utilities for production-grade handling
// Part of Core FS tools hardening - pre-IPC TODO

use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::os::unix::fs::MetadataExt;

/// Maximum file size for reading (100MB default)
pub const MAX_FILE_SIZE: u64 = 100 * 1024 * 1024;

/// Maximum file size for writing (50MB default)  
pub const MAX_WRITE_SIZE: u64 = 50 * 1024 * 1024;

/// Line ending types
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LineEnding {
    Lf,     // Unix: \n
    CrLf,   // Windows: \r\n
    Cr,     // Old Mac: \r (rare)
    Mixed,  // Mixed line endings detected
}

/// File encoding types we support
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FileEncoding {
    Utf8,
    Utf8Bom,
    Ascii,
    Binary,
    Unknown,
}

/// Symlink handling policy
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SymlinkPolicy {
    Follow,     // Follow symlinks (default for reads)
    Error,      // Error on symlinks (default for writes)
    Preserve,   // Preserve symlink (for copy operations)
}

/// File info including encoding, line endings, size
#[derive(Debug)]
pub struct FileInfo {
    pub path: PathBuf,
    pub size: u64,
    pub encoding: FileEncoding,
    pub line_ending: Option<LineEnding>,
    pub is_symlink: bool,
    pub is_binary: bool,
    pub is_readonly: bool,
}

/// Check if path is a symlink
pub fn is_symlink(path: &Path) -> io::Result<bool> {
    let metadata = fs::symlink_metadata(path)?;
    Ok(metadata.file_type().is_symlink())
}

/// Resolve symlink with safety checks
pub fn resolve_symlink(path: &Path, policy: SymlinkPolicy) -> io::Result<PathBuf> {
    let is_link = is_symlink(path)?;
    
    match (is_link, policy) {
        (false, _) => Ok(path.to_path_buf()),
        (true, SymlinkPolicy::Follow) => {
            // Follow but limit depth to prevent loops
            let mut current = path.to_path_buf();
            let mut depth = 0;
            const MAX_SYMLINK_DEPTH: u32 = 10;
            
            while depth < MAX_SYMLINK_DEPTH {
                if !is_symlink(&current)? {
                    return Ok(current);
                }
                current = fs::read_link(&current)?;
                depth += 1;
            }
            
            Err(io::Error::new(
                io::ErrorKind::Other,
                format!("Symlink depth exceeded for: {}", path.display())
            ))
        }
        (true, SymlinkPolicy::Error) => {
            Err(io::Error::new(
                io::ErrorKind::Other,
                format!("Path is a symlink (not allowed): {}", path.display())
            ))
        }
        (true, SymlinkPolicy::Preserve) => Ok(path.to_path_buf()),
    }
}

/// Detect file encoding by examining BOM and content
pub fn detect_encoding(path: &Path) -> io::Result<FileEncoding> {
    let mut file = File::open(path)?;
    let mut buffer = vec![0u8; 4096];
    let bytes_read = file.read(&mut buffer)?;
    buffer.truncate(bytes_read);
    
    // Check for UTF-8 BOM
    if buffer.starts_with(&[0xEF, 0xBB, 0xBF]) {
        return Ok(FileEncoding::Utf8Bom);
    }
    
    // Check if valid UTF-8
    if std::str::from_utf8(&buffer).is_ok() {
        // Check if pure ASCII
        if buffer.iter().all(|&b| b < 128) {
            return Ok(FileEncoding::Ascii);
        }
        return Ok(FileEncoding::Utf8);
    }
    
    // Check for null bytes (binary indicator)
    if buffer.contains(&0) {
        return Ok(FileEncoding::Binary);
    }
    
    Ok(FileEncoding::Unknown)
}

/// Detect line endings in a text file
pub fn detect_line_ending(content: &str) -> Option<LineEnding> {
    let has_crlf = content.contains("\r\n");
    let has_lf = content.contains('\n');
    let has_cr = content.contains('\r');
    
    match (has_crlf, has_lf, has_cr) {
        (true, true, _) => {
            // Check if LF appears without CR (mixed)
            let lf_only = content.split("\r\n")
                .any(|s| s.contains('\n'));
            if lf_only {
                Some(LineEnding::Mixed)
            } else {
                Some(LineEnding::CrLf)
            }
        }
        (true, false, _) => Some(LineEnding::CrLf),
        (false, true, _) => Some(LineEnding::Lf),
        (false, false, true) => Some(LineEnding::Cr),
        _ => None,
    }
}

/// Normalize line endings in content
pub fn normalize_line_endings(content: &str, target: LineEnding) -> String {
    let ending = match target {
        LineEnding::Lf => "\n",
        LineEnding::CrLf => "\r\n",
        LineEnding::Cr => "\r",
        LineEnding::Mixed => "\n", // Default to LF for mixed
    };
    
    // First normalize everything to LF
    let normalized = content
        .replace("\r\n", "\n")
        .replace('\r', "\n");
    
    // Then convert to target if needed
    if target == LineEnding::Lf {
        normalized
    } else {
        normalized.replace('\n', ending)
    }
}

/// Preserve original line endings when modifying content
pub fn preserve_line_endings(original: &str, modified: &str) -> String {
    if let Some(ending) = detect_line_ending(original) {
        normalize_line_endings(modified, ending)
    } else {
        // No line endings detected, return as-is
        modified.to_string()
    }
}

/// Get comprehensive file information
pub fn get_file_info(path: &Path) -> io::Result<FileInfo> {
    let metadata = fs::symlink_metadata(path)?;
    let is_symlink = metadata.file_type().is_symlink();
    
    // Get real metadata if following symlinks
    let real_metadata = if is_symlink {
        fs::metadata(path).unwrap_or(metadata.clone())
    } else {
        metadata.clone()
    };
    
    let encoding = detect_encoding(path)?;
    let is_binary = encoding == FileEncoding::Binary;
    
    let line_ending = if !is_binary {
        if let Ok(content) = fs::read_to_string(path) {
            detect_line_ending(&content)
        } else {
            None
        }
    } else {
        None
    };
    
    // Check if readonly on Unix
    let is_readonly = real_metadata.mode() & 0o200 == 0;
    
    Ok(FileInfo {
        path: path.to_path_buf(),
        size: real_metadata.len(),
        encoding,
        line_ending,
        is_symlink,
        is_binary,
        is_readonly,
    })
}

/// Check file size against limit
pub fn check_file_size(path: &Path, max_size: u64) -> io::Result<()> {
    let metadata = fs::metadata(path)?;
    if metadata.len() > max_size {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!(
                "File '{}' exceeds size limit: {} bytes > {} bytes",
                path.display(),
                metadata.len(),
                max_size
            )
        ));
    }
    Ok(())
}

/// Read file with size limit and encoding detection
pub fn read_file_safe(path: &Path, max_size: u64) -> io::Result<(String, FileInfo)> {
    // Check symlinks
    let resolved_path = resolve_symlink(path, SymlinkPolicy::Follow)?;
    
    // Check size
    check_file_size(&resolved_path, max_size)?;
    
    // Get file info
    let info = get_file_info(&resolved_path)?;
    
    if info.is_binary {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Cannot read binary file: {}", path.display())
        ));
    }
    
    // Handle BOM if present
    let mut content = fs::read_to_string(&resolved_path)?;
    if info.encoding == FileEncoding::Utf8Bom {
        // Strip BOM if present
        content = content.trim_start_matches('\u{FEFF}').to_string();
    }
    
    Ok((content, info))
}

/// Write file with encoding preservation and safety checks
pub fn write_file_safe(
    path: &Path, 
    content: &str,
    preserve_encoding: bool,
    max_size: u64,
) -> io::Result<()> {
    // Check content size
    if content.len() as u64 > max_size {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!(
                "Content exceeds size limit: {} bytes > {} bytes",
                content.len(),
                max_size
            )
        ));
    }
    
    // Check symlinks (don't allow writing to symlinks by default)
    if is_symlink(path)? {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!("Cannot write to symlink: {}", path.display())
        ));
    }
    
    // Preserve encoding if file exists
    let final_content = if preserve_encoding && path.exists() {
        let info = get_file_info(path)?;
        
        // Preserve BOM if present
        let with_bom = if info.encoding == FileEncoding::Utf8Bom {
            format!("\u{FEFF}{}", content)
        } else {
            content.to_string()
        };
        
        // Preserve line endings
        if let Some(ending) = info.line_ending {
            normalize_line_endings(&with_bom, ending)
        } else {
            with_bom
        }
    } else {
        content.to_string()
    };
    
    fs::write(path, final_content)
}

/// Extract specific line range from content
pub fn extract_line_range(content: &str, start: usize, end: usize) -> String {
    content.lines()
        .enumerate()
        .filter(|(idx, _)| {
            let line_num = idx + 1;
            line_num >= start && line_num <= end
        })
        .map(|(idx, line)| format!("{:4} | {}", idx + 1, line))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Apply changes only to specific line range
pub fn apply_to_line_range(
    content: &str,
    start: usize,
    end: usize,
    mut apply_fn: impl FnMut(&str) -> String,
) -> String {
    let mut result = Vec::new();
    
    for (idx, line) in content.lines().enumerate() {
        let line_num = idx + 1;
        if line_num >= start && line_num <= end {
            result.push(apply_fn(line));
        } else {
            result.push(line.to_string());
        }
    }
    
    result.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[test]
    fn test_detect_line_endings() {
        assert_eq!(detect_line_ending("hello\nworld"), Some(LineEnding::Lf));
        assert_eq!(detect_line_ending("hello\r\nworld"), Some(LineEnding::CrLf));
        assert_eq!(detect_line_ending("hello\rworld"), Some(LineEnding::Cr));
        assert_eq!(detect_line_ending("hello\r\nworld\nmixed"), Some(LineEnding::Mixed));
        assert_eq!(detect_line_ending("no newlines"), None);
    }
    
    #[test]
    fn test_normalize_line_endings() {
        let content = "line1\r\nline2\nline3\rline4";
        
        assert_eq!(
            normalize_line_endings(content, LineEnding::Lf),
            "line1\nline2\nline3\nline4"
        );
        
        assert_eq!(
            normalize_line_endings(content, LineEnding::CrLf),
            "line1\r\nline2\r\nline3\r\nline4"
        );
    }
    
    #[test]
    fn test_extract_line_range() {
        let content = "line1\nline2\nline3\nline4\nline5";
        let extracted = extract_line_range(content, 2, 4);
        
        assert!(extracted.contains("   2 | line2"));
        assert!(extracted.contains("   3 | line3"));
        assert!(extracted.contains("   4 | line4"));
        assert!(!extracted.contains("line1"));
        assert!(!extracted.contains("line5"));
    }
    
    #[test]
    fn test_apply_to_line_range() {
        let content = "foo\nbar\nbaz\nqux";
        let result = apply_to_line_range(content, 2, 3, |line| {
            line.to_uppercase()
        });
        
        let lines: Vec<&str> = result.lines().collect();
        assert_eq!(lines[0], "foo");
        assert_eq!(lines[1], "BAR");
        assert_eq!(lines[2], "BAZ");
        assert_eq!(lines[3], "qux");
    }
    
    #[test]
    fn test_symlink_detection() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("file.txt");
        let link_path = temp_dir.path().join("link.txt");
        
        fs::write(&file_path, "content").unwrap();
        
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(&file_path, &link_path).unwrap();
            assert!(is_symlink(&link_path).unwrap());
            assert!(!is_symlink(&file_path).unwrap());
        }
    }
    
    #[test]
    fn test_file_size_check() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("file.txt");
        
        fs::write(&file_path, "small content").unwrap();
        
        // Should pass with large limit
        assert!(check_file_size(&file_path, 1024).is_ok());
        
        // Should fail with tiny limit
        assert!(check_file_size(&file_path, 1).is_err());
    }
    
    #[test]
    fn test_encoding_detection() {
        let temp_dir = TempDir::new().unwrap();
        
        // UTF-8 file
        let utf8_path = temp_dir.path().join("utf8.txt");
        fs::write(&utf8_path, "Hello 世界").unwrap();
        assert_eq!(detect_encoding(&utf8_path).unwrap(), FileEncoding::Utf8);
        
        // ASCII file
        let ascii_path = temp_dir.path().join("ascii.txt");
        fs::write(&ascii_path, "Hello World").unwrap();
        assert_eq!(detect_encoding(&ascii_path).unwrap(), FileEncoding::Ascii);
        
        // UTF-8 with BOM
        let bom_path = temp_dir.path().join("bom.txt");
        fs::write(&bom_path, b"\xEF\xBB\xBFHello").unwrap();
        assert_eq!(detect_encoding(&bom_path).unwrap(), FileEncoding::Utf8Bom);
        
        // Binary file
        let binary_path = temp_dir.path().join("binary.bin");
        fs::write(&binary_path, &[0u8, 1, 2, 3, 255]).unwrap();
        assert_eq!(detect_encoding(&binary_path).unwrap(), FileEncoding::Binary);
    }
}
