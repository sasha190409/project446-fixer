//! Process kill + system helpers — all WinAPI, no taskkill/cmd/powershell.

use anyhow::Result;
use std::path::Path;
use std::time::Duration;
use std::os::windows::process::CommandExt;

use windows::core::PCWSTR;
use windows::Win32::Foundation::CloseHandle;
use windows::Win32::System::Diagnostics::Debug::MessageBeep;
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW,
    PROCESSENTRY32W, TH32CS_SNAPPROCESS,
};
use windows::Win32::System::Threading::{OpenProcess, TerminateProcess, PROCESS_TERMINATE};
use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::UI::WindowsAndMessaging::{MB_ICONEXCLAMATION, SW_SHOWNORMAL};

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

pub fn kill_by_name(name: &str) -> usize {
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

                if exe.eq_ignore_ascii_case(name) {
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

pub fn kill_csgo_with_message(msgs: &Messages) {
    println!();
    println!("{}{}{}", YELLOW, msgs.killing_csgo, RESET);
    let n: usize = CSGO_TARGETS.iter().map(|t| kill_by_name(t)).sum();
    tracing::info!(killed = n, "killed csgo processes");
    std::thread::sleep(Duration::from_millis(400));
}

pub fn kill_with_message(msgs: &Messages) {
    println!();
    println!("{}{}{}", YELLOW, msgs.killing, RESET);
    let n: usize = CSGO_TARGETS
        .iter()
        .chain(STEAM_TARGETS.iter())
        .map(|t| kill_by_name(t))
        .sum();
    tracing::info!(killed = n, "killed steam/csgo processes");
    std::thread::sleep(Duration::from_millis(400));
}

pub fn kill_all() -> Result<()> {
    for name in CSGO_TARGETS.iter().chain(STEAM_TARGETS.iter()) {
        let _ = kill_by_name(name);
    }
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

#[link(name = "dnsapi")]
extern "system" {
    fn DnsFlushResolverCache() -> i32;
}

pub fn flush_dns() -> bool {
    unsafe { DnsFlushResolverCache() != 0 }
}