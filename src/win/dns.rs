//! Cloudflare DNS + DoH configuration. Port of :ConfigureDNS.

use anyhow::{Context, Result};
use std::path::PathBuf;

use crate::ansi::*;
use crate::i18n::Messages;

pub fn configure(msgs: &Messages) -> Result<()> {
    println!();
    println!("{}{}{}", CYAN, msgs.dns_header, RESET);

    let Some(iface) = detect_active_adapter()? else {
        println!("{}{}{}", YELLOW, msgs.dns_noadapter, RESET);
        return Ok(());
    };
    println!("{}{}{}{}", GRAY, msgs.dns_adapter, iface, RESET);

    // Пункт 6: снимок текущих DNS — чтобы пользователь мог откатиться.
    if let Err(e) = snapshot_dns(&iface) {
        tracing::warn!(error = %e, "failed to snapshot DNS state (continuing)");
    }

    println!("{}{}{}", GRAY, msgs.dns_set, RESET);

    crate::ui::ensure_not_cancelled()?;
    run_netsh(&[
        "interface", "ipv4", "set", "dnsservers",
        &format!("name={}", iface),
        "source=static", "address=1.1.1.1", "register=primary",
    ])?;

    crate::ui::ensure_not_cancelled()?;
    run_netsh(&[
        "interface", "ipv4", "add", "dnsservers",
        &format!("name={}", iface),
        "address=1.0.0.1", "index=2",
    ])?;

    crate::ui::ensure_not_cancelled()?;
    run_netsh(&[
        "interface", "ipv6", "set", "dnsservers",
        &format!("name={}", iface),
        "source=static", "address=2606:4700:4700::1111", "register=primary",
    ])?;

    crate::ui::ensure_not_cancelled()?;
    run_netsh(&[
        "interface", "ipv6", "add", "dnsservers",
        &format!("name={}", iface),
        "address=2606:4700:4700::1001", "index=2",
    ])?;

    crate::ui::ensure_not_cancelled()?;

    // DoH, если поддерживается
    let supported = crate::win::process::hidden_command("powershell")
        .args(["-NoProfile", "-Command",
               "if (Get-Command Add-DnsClientDohServerAddress -ErrorAction SilentlyContinue) { exit 0 } else { exit 1 }"])
        .status()
        .map(|s| s.success()).unwrap_or(false);

    crate::ui::ensure_not_cancelled()?;

    if supported {
        println!("{}{}{}", GRAY, msgs.dns_doh_try, RESET);

        let ps = "[Console]::OutputEncoding=[System.Text.Encoding]::UTF8; \
                  $ErrorActionPreference='SilentlyContinue'; \
                  $tpl = 'https://cloudflare-dns.com/dns-query'; \
                  $addrs = @('1.1.1.1','1.0.0.1','2606:4700:4700::1111','2606:4700:4700::1001'); \
                  foreach ($a in $addrs) { \
                    Remove-DnsClientDohServerAddress -ServerAddress $a | Out-Null; \
                    Add-DnsClientDohServerAddress -ServerAddress $a -DohTemplate $tpl -AllowFallbackToUdp $true -AutoUpgrade $true | Out-Null; \
                  }; \
                  $installed = @(Get-DnsClientDohServerAddress | Where-Object { $_.DohTemplate -eq $tpl }); \
                  if ($installed.Count -ge 1) { exit 0 } else { exit 1 }";

        let output = crate::win::process::hidden_command("powershell")
            .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", ps])
            .output();

        match output {
            Ok(out) if out.status.success() => {
                println!("{}{}{}", GREEN, msgs.dns_doh_ok, RESET);
            }
            Ok(out) => {
                let stderr = String::from_utf8_lossy(&out.stderr);
                tracing::warn!(code = ?out.status.code(), stderr = %stderr.trim(),
                               "DoH state check reported no Cloudflare entries");
                println!("{}{}{}", YELLOW, msgs.dns_doh_fail, RESET);
            }
            Err(e) => {
                tracing::warn!(error = %e, "failed to launch PowerShell for DoH");
                println!("{}{}{}", YELLOW, msgs.dns_doh_fail, RESET);
            }
        }
    } else {
        println!("{}{}{}", YELLOW, msgs.dns_doh_unsup, RESET);
    }

    crate::ui::ensure_not_cancelled()?;

    if crate::win::process::flush_dns() {
        println!("{}{}{}", GRAY, msgs.dns_flush, RESET);
    }
    println!("{}{}{}", GREEN, msgs.dns_done, RESET);
    println!();
    Ok(())
}

/// Пункт 6: сброс DNS адаптера обратно на DHCP + отключение DoH.
/// Best-effort: ошибки логируются, но не прерывают вызывающего.
pub fn reset_to_dhcp() -> Result<()> {
    let Some(iface) = detect_active_adapter()? else {
        anyhow::bail!("no active adapter");
    };

    for args in [
        vec!["interface", "ipv4", "set", "dnsservers",
             &format!("name={}", iface), "source=dhcp"],
        vec!["interface", "ipv6", "set", "dnsservers",
             &format!("name={}", iface), "source=dhcp"],
    ] {
        if let Err(e) = run_netsh(&args) {
            tracing::warn!(args = ?args, error = %e, "netsh reset failed");
        }
    }

    let ps = "[Console]::OutputEncoding=[System.Text.Encoding]::UTF8; \
              $ErrorActionPreference='SilentlyContinue'; \
              $tpl = 'https://cloudflare-dns.com/dns-query'; \
              Get-DnsClientDohServerAddress | \
                Where-Object { $_.DohTemplate -eq $tpl } | \
                ForEach-Object { Remove-DnsClientDohServerAddress -ServerAddress $_.ServerAddress -Confirm:$false } | Out-Null; \
              exit 0";
    let _ = crate::win::process::hidden_command("powershell")
        .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", ps])
        .output();

    let _ = crate::win::process::flush_dns();
    Ok(())
}

/// Сохраняет текущее состояние `netsh ... show dnsservers` в
/// `<exe_dir>/backups/dns_<adapter>_<ts>.txt`. Best-effort.
fn snapshot_dns(iface: &str) -> Result<()> {
    let Some(root) = crate::win::backup_root() else {
        anyhow::bail!("no backup root");
    };
    let ts = chrono::Local::now().format("%Y%m%d_%H%M%S");
    // Заменяем пробелы/небезопасные символы в имени адаптера.
    let safe: String = iface
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect();
    let dst: PathBuf = root.join(format!("dns_{}_bak_{}.txt", safe, ts));

    let mut text = String::new();
    for family in ["ipv4", "ipv6"] {
        let out = crate::win::process::hidden_command("netsh")
            .args(["interface", family, "show", "dnsservers", &format!("name={}", iface)])
            .output()
            .context("netsh show dnsservers")?;
        text.push_str(&format!("=== {} ===\n", family));
        text.push_str(&String::from_utf8_lossy(&out.stdout));
        text.push('\n');
    }
    std::fs::write(&dst, text).context("write dns snapshot")?;
    tracing::info!(path = %dst.display(), "DNS snapshot saved");
    Ok(())
}

fn detect_active_adapter() -> Result<Option<String>> {
    let ps = "[Console]::OutputEncoding=[System.Text.Encoding]::UTF8; \
              Get-NetAdapter | Where-Object {$_.Status -eq 'Up' -and $_.InterfaceDescription -notmatch 'VPN|TAP|ZeroTier|Radmin|Hoxx|outline|Virtual|Host-Only'} | \
              Select-Object -First 1 -ExpandProperty Name";
    let out = crate::win::process::hidden_command("powershell")
        .args(["-NoProfile", "-Command", ps])
        .output().context("spawn powershell")?;
    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
    Ok(if s.is_empty() { None } else { Some(s) })
}

fn run_netsh(args: &[&str]) -> Result<()> {
    let output = crate::win::process::hidden_command("netsh")
        .args(args)
        .output()
        .with_context(|| format!("run netsh {:?}", args))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("netsh {:?} failed with {}: {}",
                      args, output.status, stderr.trim());
    }
    Ok(())
}
