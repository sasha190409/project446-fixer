//! Fix 1: stuck update. Port of :FixUpdate.

use anyhow::{bail, Context, Result};
use std::io::Read;
use std::path::Path;
use std::time::Duration;
use sha2::{Digest, Sha256};

use crate::ansi::*;
use crate::i18n::Messages;
use crate::manifest::Manifest;
use crate::url::build_full_url;
use crate::win::{disk, process};

const MIRRORS: &[&str] = &[
    "https://gc.project446.com/api/update/manifest?platform=win32",
    "https://raw.githubusercontent.com/SMorganYu/csgo_gc_public/refs/heads/main/updates/win32/manifest.txt",
    "https://gitverse.ru/api/repos/smorganyu/csgo_gc_public/raw/branch/main/updates%2Fwin32%2Fmanifest.txt",
];

/// Bail out of the current fix if the user requested cancellation.
/// Prints a status line, shows the Continue modal, and returns `Ok(())`
/// from the enclosing function.
macro_rules! bail_if_cancelled {
    ($msgs:expr) => {
        if crate::ui::is_cancelled() {
            println!("{}{}{}", YELLOW, $msgs.cancelled, RESET);
            crate::ui::pause();
            return Ok(());
        }
    };
}

pub fn run(game: &Path, msgs: &Messages, _yes: bool) -> Result<()> {
    process::kill_csgo_with_message(msgs);
    println!("{}{}{}", CYAN, msgs.fix_update, RESET);

    let (mirror, text) = fetch_manifest(msgs)?;
    bail_if_cancelled!(msgs);

    let manifest = Manifest::parse(&text).context("parse manifest")?;
    manifest.validate()?;

    println!("{}{}", msgs.version, manifest.version.as_deref().unwrap_or("?"));
    println!("{}{}", msgs.size, manifest.size.unwrap_or(0));

    // Disk space: size/1MB + 50, matching the batch.
    let need_mb = manifest.size.unwrap_or(0) / 1_048_576 + 50;
    if !disk::has_free_mb(game, need_mb) {
        println!("{}{}{}", RED, msgs.disk_low, RESET);
        crate::ui::pause();
        return Ok(());
    }

    let rel = manifest
        .url
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("manifest: missing 'url'"))?;
    let full_url = build_full_url(mirror, rel);
    tracing::info!(url = %full_url, "downloading update");

    let tmp_dir = std::env::temp_dir().join(format!(
        "csgo_fix_{}", chrono::Local::now().format("%Y%m%d_%H%M%S")));
    std::fs::create_dir_all(&tmp_dir).context("create temp dir")?;
    let gcup_tmp = tmp_dir.join("update.gcup");

    println!();
    println!("{}{}{}", CYAN, msgs.downloading, RESET);

    // Primary URL, then alternate if the first fails and we weren't cancelled.
    let body_result = download(&full_url).or_else(|_| {
        if crate::ui::is_cancelled() {
            return Err(anyhow::anyhow!("cancelled"));
        }
        let base = crate::url::mirror_base(mirror);
        let alt = format!(
            "{}/{}",
            base,
            manifest.file.as_deref().unwrap_or("update.gcup")
        );
        download(&alt)
    });

    let body = match body_result {
        Ok(b) => b,
        Err(_) => {
            if crate::ui::is_cancelled() {
                println!("{}{}{}", YELLOW, msgs.cancelled, RESET);
            } else {
                println!("{}{}{}", RED, msgs.download_fail, RESET);
                println!("{}[INFO] Temp dir kept: {}{}", GRAY, tmp_dir.display(), RESET);
            }
            crate::ui::pause();
            return Ok(());
        }
    };
    bail_if_cancelled!(msgs);

    std::fs::write(&gcup_tmp, &body).context("write update.gcup")?;

    if let Some(expected) = manifest.size {
        if body.len() as u64 != expected {
            println!("{}{}{}", RED, msgs.hash_fail, RESET);
            crate::ui::pause();
            return Ok(());
        }
    }
    if let Some(expected) = manifest.sha256.as_deref() {
        let actual = hex(&Sha256::digest(&body));
        if !actual.eq_ignore_ascii_case(expected) {
            println!("{}{}{}", RED, msgs.hash_fail, RESET);
            crate::ui::pause();
            return Ok(());
        }
    }
    println!("{}{}{}", GREEN, msgs.hash_ok, RESET);

    // --- Ed25519 signature verification ---
    let sig = manifest
        .sig
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("manifest: missing 'sig'"))?;

    match crate::crypto::load_public_key() {
        Ok(Some(pk)) => {
            if let Err(e) = crate::crypto::verify(&pk, &body, sig) {
                println!("{}{}{}", RED, msgs.sig_fail, RESET);
                println!("{}{:#}{}", GRAY, e, RESET);
                crate::ui::pause();
                return Ok(());
            }
            println!("{}{}{}", GREEN, msgs.sig_verified, RESET);
        }
        Ok(None) => {
            println!("{}{}{}", YELLOW, msgs.sig_skip_no_key, RESET);
        }
        Err(e) => {
            println!("{}{}{}{}", RED, msgs.sig_bad_key, e, RESET);
            crate::ui::pause();
            return Ok(());
        }
    }

    bail_if_cancelled!(msgs);

    std::fs::write(tmp_dir.join("update.gcup.sig"), sig).context("write sig")?;
    println!("{}{}{}", GREEN, msgs.sig_created, RESET);

    std::fs::copy(&gcup_tmp, game.join("update.gcup")).context("copy gcup")?;
    std::fs::copy(tmp_dir.join("update.gcup.sig"), game.join("update.gcup.sig"))
        .context("copy sig")?;
    if !game.join("update.gcup").exists() || !game.join("update.gcup.sig").exists() {
        println!("{}{}{}", RED, msgs.copy_fail, RESET);
        crate::ui::pause();
        return Ok(());
    }
    println!("{}{}{}", GREEN, msgs.copy_ok, RESET);

    let gc = game.join("csgo_gc");
    for name in [".update.lock", "update_staged.txt", "update_target.txt"] {
        let p = gc.join(name);
        if p.exists() {
            if let Err(e) = std::fs::remove_file(&p) {
                tracing::warn!(
                    path = %p.display(),
                    error = %e,
                    "failed to remove stale update file"
                );
            }
        }
    }
    match crate::win::registry::delete_user_env("GC_UPDATE_DISABLE") {
        Ok(true) => tracing::info!("removed GC_UPDATE_DISABLE"),
        Ok(false) => {}
        Err(e) => tracing::warn!(error = %e, "failed to remove GC_UPDATE_DISABLE"),
    }

    let rus = game.join("csgo").join("resource").join("csgo_gc_russian.txt");
    if rus.exists() {
        if let Some(bk) = backup_root() {
            let ts = chrono::Local::now().format("%Y%m%d_%H%M%S");
            let dst = bk.join(format!("csgo_gc_russian.txt.bak_{}", ts));

            if let Err(e) = std::fs::copy(&rus, &dst) {
                tracing::warn!(
                    source = %rus.display(),
                    destination = %dst.display(),
                    error = %e,
                    "failed to back up csgo_gc_russian.txt"
                );
            }
        }

        if let Err(e) = std::fs::remove_file(&rus) {
            tracing::warn!(
                path = %rus.display(),
                error = %e,
                "failed to remove csgo_gc_russian.txt"
            );
        }
    }

    let _ = std::fs::remove_dir_all(&tmp_dir);
    println!();
    println!("{}{}{}", GREEN, msgs.update_done, RESET);
    Ok(())
}

fn fetch_manifest(msgs: &Messages) -> Result<(&'static str, String)> {
    let mut last_err = None;
    for mirror in MIRRORS {
        match ureq::get(*mirror).timeout(Duration::from_secs(30)).call() {
            Ok(resp) => match resp.into_string() {
                Ok(text) if !text.trim().is_empty() => return Ok((mirror, text)),
                Ok(_) => last_err = Some(format!("{}: empty body", mirror)),
                Err(e) => last_err = Some(format!("{}: {}", mirror, e)),
            },
            Err(e) => last_err = Some(format!("{}: {}", mirror, e)),
        }
    }
    println!("{}{}{}", RED, msgs.manifest_fail, RESET);
    bail!("all mirrors failed: {}", last_err.unwrap_or_default())
}

/// Download `url` in 64 KiB chunks, reporting progress and honouring cancel.
fn download(url: &str) -> Result<Vec<u8>> {
    // Clear any leftover bar from a previous attempt.
    crate::ui::progress(0, 0);

    let resp = ureq::get(url)
        .timeout(Duration::from_secs(900))
        .call()
        .with_context(|| format!("GET {}", url))?;

    let total = resp
        .header("Content-Length")
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0);

    if total > 0 {
        crate::ui::progress(0, total);
    }

    let mut reader = resp.into_reader();
    let cap = (total as usize).min(64 << 20).max(1 << 20);
    let mut out = Vec::with_capacity(cap);
    let mut buf = [0u8; 64 * 1024];
    let mut done = 0u64;

    loop {
        if crate::ui::is_cancelled() {
            crate::ui::progress(0, 0);
            bail!("cancelled by user");
        }
        let n = reader.read(&mut buf).context("read body")?;
        if n == 0 {
            break;
        }
        out.extend_from_slice(&buf[..n]);
        done += n as u64;
        if total > 0 {
            crate::ui::progress(done, total);
        }
    }

    crate::ui::progress(0, 0);
    Ok(out)
}

fn backup_root() -> Option<std::path::PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let dir = exe.parent()?.join("backups");
    std::fs::create_dir_all(&dir).ok()?;
    Some(dir)
}

fn hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{:02x}", b));
    }
    s
}