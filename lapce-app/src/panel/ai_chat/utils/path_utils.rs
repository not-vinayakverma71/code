// Path utilities - ported from utils/removeLeadingNonAlphanumeric.ts
pub fn remove_leading_non_alphanumeric(s: &str) -> &str {
    s.trim_start_matches(|c: char| !c.is_alphanumeric())
}

pub fn format_path(path: &str) -> String {
    if path.starts_with("./") || path.starts_with(".\\") {
        path[2..].to_string()
    } else {
        path.to_string()
    }
}
