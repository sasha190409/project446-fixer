//! Antivirus exclusions + product listing. Port of :AddAVExclusions.


use crate::ansi::*;
use crate::i18n::Messages;

pub fn run(msgs: &Messages) {
    if crate::ui::is_cancelled() {
        return;
    }
    println!();
    println!("{}{}{}", CYAN, msgs.av_check, RESET);

    let ps = "$h = Join-Path $env:SystemRoot 'System32\\drivers\\etc\\hosts';\
              $r1 = $false;\
              try { Add-MpPreference -ExclusionPath $h -ErrorAction Stop; $r1 = $true } catch { };\
              if ($r1) { Write-Output 'DEFENDER_OK' } else { Write-Output 'DEFENDER_FAIL' };\
              try { Get-CimInstance -Namespace 'root\\SecurityCenter2' -ClassName AntiVirusProduct -ErrorAction Stop | ForEach-Object { Write-Output ('AV:' + $_.displayName) } } catch { }";

    let out = crate::win::process::hidden_command("powershell")
        .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command",
               &format!("[Console]::OutputEncoding=[System.Text.Encoding]::UTF8; {}", ps)])
        .output();

    let mut defender_ok = false;
    let mut av_found = false;

    if let Ok(out) = out {
        for line in String::from_utf8_lossy(&out.stdout).lines() {
            if line == "DEFENDER_OK"   { defender_ok = true; }
            if line == "DEFENDER_FAIL" { defender_ok = false; }
            if let Some(name) = line.strip_prefix("AV:") {
                av_found = true;
                println!("  - {}", name.trim());
            }
        }
    }

    if defender_ok {
        println!("{}{}{}", GREEN, msgs.defender_ok, RESET);
    } else {
        println!("{}{}{}", RED, msgs.defender_fail, RESET);
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