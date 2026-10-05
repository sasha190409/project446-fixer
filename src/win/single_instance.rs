// win/single_instance.rs
use windows::core::w;
use windows::Win32::Foundation::{CloseHandle, GetLastError, HANDLE, ERROR_ALREADY_EXISTS};
use windows::Win32::System::Threading::CreateMutexW;

pub struct InstanceGuard(HANDLE);
impl Drop for InstanceGuard {
    fn drop(&mut self) { unsafe { let _ = CloseHandle(self.0); } }
}

pub fn acquire() -> Option<InstanceGuard> {
    unsafe {
        let h = CreateMutexW(None, true, w!("Global\\CSGOLegacyFixer")).ok()?;
        if GetLastError() == ERROR_ALREADY_EXISTS {
            let _ = CloseHandle(h);
            return None;
        }
        Some(InstanceGuard(h))
    }
}
