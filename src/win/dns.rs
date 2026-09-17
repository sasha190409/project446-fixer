//! Cloudflare DNS + DoH configuration. Port of :ConfigureDNS.
//!
//! Forces PowerShell to output UTF-8 so adapter names with Cyrillic
//! survive the pipe.

use anyhow::{Context, Result};

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
    println!("{}{}{}", GRAY, msgs.dns_set, RESET);

    crate::ui::ensure_not_cancelled()?;
    run_netsh(&[
        "interface",
        "ipv4",
        "set",
        "dnsservers",
        &format!("name={}", iface),
        "source=static",
        "address=1.1.1.1",
        "register=primary",
    ])?;

    crate::ui::ensure_not_cancelled()?;
    run_netsh(&[
        "interface",
        "ipv4",
        "add",
        "dnsservers",
        &format!("name={}", iface),
        "address=1.0.0.1",
        "index=2",
    ])?;

    crate::ui::ensure_not_cancelled()?;
    run_netsh(&[
        "interface",
        "ipv6",
        "set",
        "dnsservers",
        &format!("name={}", iface),
        "source=static",
        "address=2606:4700:4700::1111",
        "register=primary",
    ])?;

    crate::ui::ensure_not_cancelled()?;
    run_netsh(&[
        "interface",
        "ipv6",
        "add",
        "dnsservers",
        &format!("name={}", iface),
        "address=2606:4700:4700::1001",
        "index=2",
    ])?;

    crate::ui::ensure_not_cancelled()?;

    // DoH if supported
    let supported = crate::win::process::hidden_command("powershell")
        .args(["-NoProfile", "-Command",
               "if (Get-Command Add-DnsClientDohServerAddress -ErrorAction SilentlyContinue) { exit 0 } else { exit 1 }"])
        .status()
        .map(|s| s.success()).unwrap_or(false);

    crate::ui::ensure_not_cancelled()?;

    if supported {
        println!("{}{}{}", GRAY, msgs.dns_doh_try, RESET);
        let ps = "$ErrorActionPreference='SilentlyContinue';\
            Add-DnsClientDohServerAddress -ServerAddress '1.1.1.1' -DohTemplate 'https://cloudflare-dns.com/dns-query' -AllowFallbackToUdp $true -AutoUpgrade $true;\
            Add-DnsClientDohServerAddress -ServerAddress '1.0.0.1' -DohTemplate 'https://cloudflare-dns.com/dns-query' -AllowFallbackToUdp $true -AutoUpgrade $true;\
            Add-DnsClientDohServerAddress -ServerAddress '2606:4700:4700::1111' -DohTemplate 'https://cloudflare-dns.com/dns-query' -AllowFallbackToUdp $true -AutoUpgrade $true;\
            Add-DnsClientDohServerAddress -ServerAddress '2606:4700:4700::1001' -DohTemplate 'https://cloudflare-dns.com/dns-query' -AllowFallbackToUdp $true -AutoUpgrade $true";
        match crate::win::process::hidden_command("powershell")
            .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", ps])
            .status()
        {
            Ok(status) if status.success() => {
                println!("{}{}{}", GREEN, msgs.dns_doh_ok, RESET);
            }
            Ok(status) => {
                tracing::warn!(status = ?status, "DoH configuration failed");
                println!("{}{}{}", YELLOW, msgs.dns_doh_unsup, RESET);
            }
            Err(e) => {
                tracing::warn!(error = %e, "failed to launch PowerShell for DoH");
                println!("{}{}{}", YELLOW, msgs.dns_doh_unsup, RESET);
            }
        }
    } else {
        println!("{}{}{}", YELLOW, msgs.dns_doh_unsup, RESET);
    }

    crate::ui::ensure_not_cancelled()?;

    if crate::win::process::flush_dns() {
        println!("{}{}{}", GRAY, msgs.dns_flush, RESET);
    } else {
        tracing::warn!("DnsFlushResolverCache returned 0");
    }
    println!("{}{}{}", GREEN, msgs.dns_done, RESET);
    println!();
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
        anyhow::bail!(
            "netsh {:?} failed with {}: {}",
            args,
            output.status,
            stderr.trim()
        );
    }

    Ok(())
}