//! Process kill + system helpers — all WinAPI, no taskkill/cmd/powershell.

use anyhow::Result;
use std::path::Path;
use std::time::Duration;
use std::os::windows::process::CommandExt;

use windows::core::PCWSTR;
use windows::Win32::Foundation::{CloseHandle, BOOL, HWND, LPARAM};
use windows::Win32::System::Diagnostics::Debug::MessageBeep;
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW,
    PROCESSENTRY32W, TH32CS_SNAPPROCESS,
};
use windows::Win32::System::Threading::{OpenProcess, TerminateProcess, PROCESS_TERMINATE};
use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetWindowTextLengthW, GetWindowTextW, IsWindowVisible,
    SetForegroundWindow, ShowWindow, MB_ICONEXCLAMATION, SW_RESTORE, SW_SHOWNORMAL,
};

use crate::ansi::*;
use crate::i18n::Messages;

const CSGO_TARGETS: &[&str] = &["csgo.exe"];

const STEAM_TARGETS: &[&str] = &[
    "steam.exe",
    "steamwebhelper.exe",
    "steamservice.exe",
    "steamcrashhandler.exe",
];

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

pub fn hidden_command(program: &str) -> std::process::Command {
    let mut cmd = std::process::Command::new(program);
    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);
    cmd
}

/// Kill every process whose exe name matches any entry in `names`,
/// using a single process snapshot.
pub fn kill_by_names(names: &[&str]) -> usize {
    unsafe {
        let snapshot = match CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) {
            Ok(h) => h,
            Err(_) => return 0,
        };

        let mut entry = PROCESSENTRY32W {
            dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
            ..Default::default()
        };

        let mut killed = 0usize;

        if Process32FirstW(snapshot, &mut entry).is_ok() {
            loop {
                let end = entry
                    .szExeFile
                    .iter()
                    .position(|&c| c == 0)
                    .unwrap_or(entry.szExeFile.len());
                let exe = String::from_utf16_lossy(&entry.szExeFile[..end]);

                if names.iter().any(|n| exe.eq_ignore_ascii_case(n)) {
                    if let Ok(handle) =
                        OpenProcess(PROCESS_TERMINATE, false, entry.th32ProcessID)
                    {
                        let _ = TerminateProcess(handle, 1);
                        let _ = CloseHandle(handle);
                        killed += 1;
                    }
                }

                if Process32NextW(snapshot, &mut entry).is_err() {
                    break;
                }
            }
        }

        let _ = CloseHandle(snapshot);
        killed
    }
}

/// Single-target convenience wrapper. Kept for callers outside this module.
pub fn kill_by_name(name: &str) -> usize {
    kill_by_names(&[name])
}

pub fn kill_csgo_with_message(msgs: &Messages) {
    println!();
    println!("{}{}{}", YELLOW, msgs.killing_csgo, RESET);
    let n = kill_by_names(CSGO_TARGETS);
    tracing::info!(killed = n, "killed csgo processes");
    std::thread::sleep(Duration::from_millis(400));
}

pub fn kill_with_message(msgs: &Messages) {
    println!();
    println!("{}{}{}", YELLOW, msgs.killing, RESET);
    let all: Vec<&str> = CSGO_TARGETS
        .iter()
        .chain(STEAM_TARGETS.iter())
        .copied()
        .collect();
    let n = kill_by_names(&all);
    tracing::info!(killed = n, "killed steam/csgo processes");
    std::thread::sleep(Duration::from_millis(400));
}

pub fn kill_all() -> Result<()> {
    let all: Vec<&str> = CSGO_TARGETS
        .iter()
        .chain(STEAM_TARGETS.iter())
        .copied()
        .collect();
    let _ = kill_by_names(&all);
    std::thread::sleep(Duration::from_millis(400));
    Ok(())
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

pub fn shell_open(target: &str) -> bool {
    unsafe {
        let verb = wide("open");
        let file = wide(target);
        let ret = ShellExecuteW(
            None,
            PCWSTR(verb.as_ptr()),
            PCWSTR(file.as_ptr()),
            PCWSTR::null(),
            PCWSTR::null(),
            SW_SHOWNORMAL,
        );
        ret.0 as usize > 32
    }
}

pub fn shell_open_folder(folder: &Path) -> bool {
    shell_open(&folder.to_string_lossy())
}

pub fn beep() {
    unsafe {
        let _ = MessageBeep(MB_ICONEXCLAMATION);
    }
}

// ---------------------------------------------------------------------------
// Window focusing
// ---------------------------------------------------------------------------

struct FindCtx {
    found: Option<HWND>,
    needle_lower: String,
}

unsafe extern "system" fn find_window_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let ctx = &mut *(lparam.0 as *mut FindCtx);

    if !IsWindowVisible(hwnd).as_bool() {
        return BOOL(1);
    }
    let len = GetWindowTextLengthW(hwnd);
    if len <= 0 {
        return BOOL(1);
    }
    let mut buf = vec![0u16; (len + 1) as usize];
    let n = GetWindowTextW(hwnd, &mut buf);
    if n <= 0 {
        return BOOL(1);
    }
    let title = String::from_utf16_lossy(&buf[..n as usize]);
    let title_lower = title.to_lowercase();

    // Exact match or prefix on "Steam" — the Steam main window title is
    // literally "Steam" on all current clients, and any overlay/popup is
    // unlikely to also start with that token in a visible top-level window.
    if title_lower == ctx.needle_lower
        || title_lower.starts_with(&ctx.needle_lower)
    {
        ctx.found = Some(hwnd);
        return BOOL(0);
    }
    BOOL(1)
}

/// Find a visible top-level window whose title equals `needle` (case-
/// insensitive) or starts with it, restore it, and try to bring it to the
/// foreground. Returns `true` on success.
pub fn focus_window_by_title(needle: &str) -> bool {
    let mut ctx = FindCtx {
        found: None,
        needle_lower: needle.to_lowercase(),
    };
    unsafe {
        let _ = EnumWindows(
            Some(find_window_proc),
            LPARAM(&mut ctx as *mut _ as isize),
        );
    }

    let Some(hwnd) = ctx.found else {
        return false;
    };
    unsafe {
        let _ = ShowWindow(hwnd, SW_RESTORE);
        SetForegroundWindow(hwnd).as_bool()
    }
}

pub fn focus_steam_window() -> bool {
    focus_window_by_title("Steam")
}

// ---------------------------------------------------------------------------
// DNS
// ---------------------------------------------------------------------------

#[link(name = "dnsapi")]
extern "system" {
    fn DnsFlushResolverCache() -> i32;
}

pub fn flush_dns() -> bool {
    unsafe { DnsFlushResolverCache() != 0 }
}
