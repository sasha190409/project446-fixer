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

        // Idempotent script. Key rules:
        //   1. Remove before Add so re-runs never hit
        //      DohServerAlreadyExistsException.
        //   2. SilentlyContinue on every Add — a second-run "already exists"
        //      is not a failure for us, it's the target state.
        //   3. Final verdict comes from *reading state*, not from the exit
        //      code of a cmdlet. `powershell.exe -Command` derives its exit
        //      code from `$?` of the last statement, which is fragile when
        //      we intentionally tolerate errors.
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
                tracing::warn!(
                    code = ?out.status.code(),
                    stderr = %stderr.trim(),
                    "DoH state check reported no Cloudflare entries"
                );
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
