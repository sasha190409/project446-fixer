//! Free-space check. Port of :CheckDiskSpace.

use std::path::Path;

use windows::core::PCWSTR;
use windows::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;

/// Returns true if the drive holding `path` has at least `need_mb` MB free.
/// On any error, returns true (matching the batch: "assume OK").
pub fn has_free_mb(path: &Path, need_mb: u64) -> bool {
    let Some(free_bytes) = free_bytes(path) else {
        return true;
    };
    let need_bytes = need_mb.saturating_mul(1_048_576);
    free_bytes >= need_bytes
}

fn free_bytes(path: &Path) -> Option<u64> {
    // Extract drive letter ("C:\Users\..." -> "C:\"). UNC paths and other
    // non-drive roots are treated as "unknown" -> we assume OK upstream.
    let root = match path.to_string_lossy().chars().next() {
        Some(c) if c.is_ascii_alphabetic() => format!("{}:\\", c),
        _ => return None,
    };
    let wide: Vec<u16> = root.encode_utf16().chain(std::iter::once(0)).collect();

    let mut free: u64 = 0;
    unsafe {
        GetDiskFreeSpaceExW(
            PCWSTR(wide.as_ptr()),
            Some(&mut free as *mut u64),
            None,
            None,
        )
        .ok()?;
    }
    Some(free)
}
