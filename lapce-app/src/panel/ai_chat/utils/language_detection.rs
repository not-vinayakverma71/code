// Language detection from file paths - ported from utils/getLanguageFromPath.ts
use std::path::Path;

pub fn get_language_from_path(path: &str) -> Option<&'static str> {
    let path = Path::new(path);
    let ext = path.extension()?.to_str()?;
    
    Some(match ext {
        "rs" => "rust",
        "ts" | "tsx" => "typescript",
        "js" | "jsx" => "javascript",
        "py" => "python",
        "go" => "go",
        "java" => "java",
        "c" => "c",
        "cpp" | "cc" | "cxx" => "cpp",
        "h" | "hpp" => "cpp",
        "cs" => "csharp",
        "rb" => "ruby",
        "php" => "php",
        "swift" => "swift",
        "kt" => "kotlin",
        "scala" => "scala",
        "sh" | "bash" => "bash",
        "json" => "json",
        "yaml" | "yml" => "yaml",
        "toml" => "toml",
        "xml" => "xml",
        "html" => "html",
        "css" => "css",
        "scss" | "sass" => "scss",
        "md" => "markdown",
        "sql" => "sql",
        "dockerfile" => "dockerfile",
        _ => return None,
    })
}
