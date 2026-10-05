//! Global single-instance mutex. Prevents two fixer processes from
//! fighting over hosts, csgo.exe, and %TEMP%\csgo_fix_* simultaneously.

use windows::core::w;
use windows::Win32::Foundation::{
    CloseHandle, GetLastError, ERROR_ALREADY_EXISTS, HANDLE,
};
use windows::Win32::System::Threading::CreateMutexW;

pub struct InstanceGuard(HANDLE);

impl Drop for InstanceGuard {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseHandle(self.0);
        }
    }
}

/// Returns `Some(guard)` if this is the only instance, `None` otherwise.
/// The guard releases the mutex on drop; keep it alive for the whole
/// process lifetime.
pub fn acquire() -> Option<InstanceGuard> {
    unsafe {
        let h = CreateMutexW(None, true, w!("Global\\CSGOLegacyFixer-v1")).ok()?;
        // CreateMutexW returns Ok even when the mutex already existed —
        // the only way to tell is GetLastError().
        if GetLastError() == ERROR_ALREADY_EXISTS {
            let _ = CloseHandle(h);
            return None;
        }
        Some(InstanceGuard(h))
    }
}
