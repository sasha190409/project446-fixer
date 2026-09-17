//! Steam libraryfolders.vdf path extraction.
//! Simplified port of :SearchVdf / :ProcessVdfLine without findstr.

/// Returns every `"path" "..."` value in order of appearance.
/// Handles escaped backslashes `\\`.
pub fn extract_paths(vdf: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in vdf.lines() {
        let trimmed = line.trim();
        let Some(rest) = trimmed.strip_prefix("\"path\"") else {
            continue;
        };
        let rest = rest.trim_start();
        let Some(rest) = rest.strip_prefix('"') else {
            continue;
        };
        let Some(end) = rest.find('"') else { continue };
        let raw = &rest[..end];
        out.push(raw.replace("\\\\", "\\"));
    }
    out
}