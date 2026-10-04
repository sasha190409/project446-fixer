pub mod av;
pub mod disk;
pub mod dns;
pub mod elevate;
pub mod hosts;
pub mod process;
pub mod registry;
pub mod tutorial;

use std::path::PathBuf;

/// Директория для бэкапов: `<exe_dir>/backups`.
/// Возвращает `None`, если создать её не удалось.
pub fn backup_root() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let dir = exe.parent()?.join("backups");
    std::fs::create_dir_all(&dir).ok()?;
    Some(dir)
}

// ---------------------------------------------------------------------------
// System UI language
// ---------------------------------------------------------------------------

// `GetUserDefaultUILanguage` returns the LANGID of the *UI* language the user
// chose in Settings → Time & language. It is a WINAPI/stdcall function exported
// from kernel32.dll; `extern "system"` maps to stdcall on x86 and to the default
// C calling convention on x64.
#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetUserDefaultUILanguage() -> u16;
}

/// Returns the language the app should default to when the user has not
/// yet picked one explicitly (no `Language` value in the registry key).
pub fn detect_system_lang() -> crate::args::Lang {
    // SAFETY: no arguments, returns LANGID by value, no allocation,
    // no side effects on process state. Safe to call from any thread.
    let langid = unsafe { GetUserDefaultUILanguage() };
    crate::args::Lang::from_langid(langid)
}
