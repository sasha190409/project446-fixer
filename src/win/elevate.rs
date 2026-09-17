//! Elevation check and re-launch via ShellExecuteExW("runas").

use anyhow::{Context, Result};
use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use windows::core::PCWSTR;
use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::Security::{
    GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY,
};
use windows::Win32::System::Threading::{
    GetCurrentProcess, GetExitCodeProcess, OpenProcessToken, WaitForSingleObject, INFINITE,
};
use windows::Win32::UI::Shell::{
    ShellExecuteExW, SHELLEXECUTEINFOW, SEE_MASK_NOCLOSEPROCESS,
};

fn to_wide(s: &OsStr) -> Vec<u16> {
    s.encode_wide().chain(std::iter::once(0)).collect()
}

pub fn is_elevated() -> bool {
    unsafe {
        let mut token = HANDLE::default();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token).is_err() {
            return false;
        }
        let mut elevation = TOKEN_ELEVATION::default();
        let mut ret_len = 0u32;
        let ok = GetTokenInformation(
            token,
            TokenElevation,
            Some(&mut elevation as *mut _ as *mut _),
            std::mem::size_of::<TOKEN_ELEVATION>() as u32,
            &mut ret_len,
        )
        .is_ok();
        let _ = CloseHandle(token);
        ok && elevation.TokenIsElevated != 0
    }
}

fn relaunch_elevated() -> Result<()> {
    let exe = std::env::current_exe().context("current_exe")?;
    let args: Vec<String> = std::env::args().skip(1).collect();
    let args_joined = args
        .iter()
        .map(|a| if a.contains(' ') { format!("\"{}\"", a) } else { a.clone() })
        .collect::<Vec<_>>()
        .join(" ");

    let exe_w = to_wide(exe.as_os_str());
    let args_w = to_wide(OsStr::new(&args_joined));
    let verb_w = to_wide(OsStr::new("runas"));

    unsafe {
        let mut info = SHELLEXECUTEINFOW {
            cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
            fMask: SEE_MASK_NOCLOSEPROCESS,
            lpVerb: PCWSTR(verb_w.as_ptr()),
            lpFile: PCWSTR(exe_w.as_ptr()),
            lpParameters: PCWSTR(args_w.as_ptr()),
            nShow: 1,
            ..Default::default()
        };
        ShellExecuteExW(&mut info).context("ShellExecuteExW(runas)")?;
        if !info.hProcess.is_invalid() {
            WaitForSingleObject(info.hProcess, INFINITE);
            let mut code = 0u32;
            let _ = GetExitCodeProcess(info.hProcess, &mut code);
            let _ = CloseHandle(info.hProcess);
            std::process::exit(code as i32);
        }
    }
    Ok(())
}

pub fn ensure_elevated() -> Result<()> {
    if is_elevated() {
        return Ok(());
    }
    // Bilingual on purpose: elevation happens before the language is known.
    let _ = rfd::MessageDialog::new()
        .set_title("CS:GO Legacy Fixer")
        .set_description(
            "Administrator privileges are required.\n\
             Требуются права администратора.",
        )
        .set_buttons(rfd::MessageButtons::Ok)
        .show();
    relaunch_elevated()?;
    std::process::exit(0);
}