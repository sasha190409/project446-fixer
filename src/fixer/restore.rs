//! Restore from backups: econ folder and hosts file.
//!
//! User-facing counterpart to the backups created by icons.rs and hosts.rs.
//! Best-effort: we do not attempt to merge — we replace.

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

use crate::ansi::*;
use crate::i18n::Messages;
use crate::win::process;

#[derive(Clone)]
pub struct BackupEntry {
    pub path: PathBuf,
    pub kind: BackupKind,
    pub label: String,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum BackupKind {
    Econ,
    Hosts,
}

/// Возвращает все доступные бэкапы, свежие — в начале списка.
pub fn list() -> Vec<BackupEntry> {
    let Some(root) = crate::win::backup_root() else { return Vec::new() };
    let Ok(rd) = std::fs::read_dir(&root) else { return Vec::new() };

    let mut out = Vec::new();
    for entry in rd.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with("econ.bak_") {
            out.push(BackupEntry { path: entry.path(), kind: BackupKind::Econ, label: name });
        } else if name.starts_with("hosts.bak_") {
            out.push(BackupEntry { path: entry.path(), kind: BackupKind::Hosts, label: name });
        }
    }
    out.sort_by(|a, b| b.label.cmp(&a.label));
    out
}

pub fn restore(game: &Path, entry: &BackupEntry, msgs: &Messages) -> Result<()> {
    process::kill_csgo_with_message(msgs);
    println!();
    match entry.kind {
        BackupKind::Econ  => restore_econ(game, &entry.path, msgs),
        BackupKind::Hosts => restore_hosts(&entry.path, msgs),
    }
}

fn restore_econ(game: &Path, backup: &Path, msgs: &Messages) -> Result<()> {
    let target = game.join("csgo").join("resource").join("flash").join("econ");

    // Существующую папку не удаляем — уносим рядом, чтобы пользователь
    // мог вернуть руками, если восстановление окажется не тем.
    if target.exists() {
        let ts = chrono::Local::now().format("%Y%m%d_%H%M%S");
        let aside = target.with_extension(format!("pre-restore_{}", ts));
        std::fs::rename(&target, &aside)
            .with_context(|| format!("move current econ aside -> {}", aside.display()))?;
        println!("{}{}{}{}", GRAY, msgs.restore_aside, aside.display(), RESET);
    }

    let bytes = copy_dir_counted(backup, &target).context("copy backup -> econ")?;
    println!("{}{}{}{}", GREEN, msgs.restore_ok, target.display(), RESET);
    println!("{}{}{}{}", GRAY, msgs.restore_bytes, bytes, RESET);
    Ok(())
}

fn restore_hosts(backup: &Path, msgs: &Messages) -> Result<()> {
    let root = std::env::var_os("SystemRoot")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"C:\Windows"));
    let target = root.join("System32").join("drivers").join("etc").join("hosts");

    // Бэкапим текущий hosts перед перезаписью.
    if target.exists() {
        let ts = chrono::Local::now().format("%Y%m%d_%H%M%S");
        let bak = target.with_extension(format!("bak_{}", ts));
        let _ = std::fs::copy(&target, &bak);
    }

    std::fs::copy(backup, &target)
        .with_context(|| format!("copy {} -> {}", backup.display(), target.display()))?;
    println!("{}{}{}{}", GREEN, msgs.restore_ok, target.display(), RESET);
    Ok(())
}

fn copy_dir_counted(src: &Path, dst: &Path) -> std::io::Result<u64> {
    std::fs::create_dir_all(dst)?;
    let mut total = 0u64;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let to = dst.join(entry.file_name());
        if ty.is_dir() {
            total += copy_dir_counted(&entry.path(), &to)?;
        } else {
            total += entry.metadata()?.len();
            std::fs::copy(entry.path(), to)?;
        }
    }
    Ok(total)
}
