//! Free-space check. Port of :CheckDiskSpace.

use std::path::Path;

/// Returns true if the drive holding `path` has at least `need_mb` MB free.
/// On any error, returns true (matching the batch: "assume OK").
pub fn has_free_mb(path: &Path, need_mb: u64) -> bool {
    let Some(free_bytes) = free_bytes(path) else { return true; };
    let need_bytes = need_mb.saturating_mul(1_048_576);
    free_bytes >= need_bytes
}

fn free_bytes(path: &Path) -> Option<u64> {
    let s = path.to_string_lossy();
    let drive = s.chars().next()?;
    if !drive.is_ascii_alphabetic() { return None; }
    let out = crate::win::process::hidden_command("powershell")
        .args(["-NoProfile", "-Command",
               &format!("$ErrorActionPreference='Stop'; try {{ (Get-PSDrive -Name '{}').Free }} catch {{ 'UNKNOWN' }}", drive)])
        .output().ok()?;
    let t = String::from_utf8_lossy(&out.stdout);
    t.trim().parse::<u64>().ok()
}