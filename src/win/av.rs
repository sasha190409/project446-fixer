//! Antivirus product listing + manual exclusion instructions.
//!
//! We deliberately do NOT touch Defender preferences from code. Calling
//! `Add-MpPreference -ExclusionPath` at runtime is a textbook malware
//! heuristic and reliably gets this binary flagged.

use crate::ansi::*;
use crate::i18n::Messages;

pub fn run(msgs: &Messages) {
    if crate::ui::is_cancelled() {
        return;
    }
    println!();
    println!("{}{}{}", CYAN, msgs.av_check, RESET);

    let ps = "try { \
                Get-CimInstance -Namespace 'root\\SecurityCenter2' \
                  -ClassName AntiVirusProduct -ErrorAction Stop \
                | ForEach-Object { Write-Output ('AV:' + $_.displayName) } \
              } catch { }";

    let out = crate::win::process::hidden_command("powershell")
        .args([
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            &format!("[Console]::OutputEncoding=[System.Text.Encoding]::UTF8; {}", ps),
        ])
        .output();

    let mut av_found = false;
    if let Ok(out) = out {
        for line in String::from_utf8_lossy(&out.stdout).lines() {
            if let Some(name) = line.strip_prefix("AV:") {
                av_found = true;
                println!("  - {}", name.trim());
            }
        }
    }
    if !av_found {
        println!("{}{}{}", YELLOW, msgs.av_noav, RESET);
    }

    println!();
    println!("{}{}{}", YELLOW, msgs.av_manual, RESET);
    println!("{}{}{}", GRAY, msgs.av_hdr, RESET);

    // Expand %SystemRoot% at print time — Rust literals don't do it.
    let hosts_path = std::env::var("SystemRoot")
        .map(|r| format!(r"{}\System32\drivers\etc\hosts", r))
        .unwrap_or_else(|_| r"C:\Windows\System32\drivers\etc\hosts".to_string());
    println!("{}{}{}", GRAY, hosts_path, RESET);

    if !msgs.av_p2.is_empty() {
        println!("{}{}{}", GRAY, msgs.av_p2, RESET);
    }
    println!();
}
