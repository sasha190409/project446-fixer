//! Fix 2: broken inventory icons. Port of :FixIcons.

use anyhow::Result;
use std::path::Path;

use crate::ansi::*;
use crate::i18n::Messages;
use crate::win::process;

pub fn run(game: &Path, msgs: &Messages) -> Result<()> {
    process::kill_csgo_with_message(msgs);
    println!();
    println!("{}{}{}", CYAN, msgs.fix_icons, RESET);

    let econ = game.join("csgo").join("resource").join("flash").join("econ");
    if !econ.exists() {
        println!("{}{}{}{}", RED, msgs.no_econ, econ.display(), RESET);
        crate::ui::pause();
        return Ok(());
    }

    let Some(bk) = backup_root() else {
        println!("{}{}{}", RED, msgs.backup_fail, econ.display());
        crate::ui::pause();
        return Ok(());
    };

    let ts = chrono::Local::now().format("%Y%m%d_%H%M%S");
    let dst = bk.join(format!("econ.bak_{}", ts));

    if let Err(e) = copy_dir(&econ, &dst) {
        println!(
            "{}{}{} ({}){}",
            RED,
            msgs.backup_fail,
            econ.display(),
            e,
            RESET
        );
        crate::ui::pause();
        return Ok(());
    }

    println!("{}{}{}{}", GRAY, msgs.backup_ok, dst.display(), RESET);

    let total = dir_size(&econ).unwrap_or(0);
    crate::ui::progress(0, total);

    match remove_dir_with_progress(&econ, total) {
        Ok(()) => {
            crate::ui::progress(0, 0);
            println!("{}{}{}", GREEN, msgs.rd_ok, RESET);
        }
        Err(e) => {
            crate::ui::progress(0, 0);
            println!("{}{}: {}{}", RED, msgs.rd_fail, e, RESET);
            return Err(e.into());
        }
    }
    Ok(())
}

fn backup_root() -> Option<std::path::PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let dir = exe.parent()?.join("backups");
    std::fs::create_dir_all(&dir).ok()?;
    Some(dir)
}

fn copy_dir(src: &Path, dst: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let to = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir(&entry.path(), &to)?;
        } else {
            std::fs::copy(entry.path(), to)?;
        }
    }
    Ok(())
}

fn dir_size(path: &Path) -> std::io::Result<u64> {
    let mut total = 0u64;
    for entry in std::fs::read_dir(path)? {
        let entry = entry?;
        let meta = entry.metadata()?;
        if meta.is_dir() {
            total += dir_size(&entry.path())?;
        } else {
            total += meta.len();
        }
    }
    Ok(total)
}

fn remove_dir_with_progress(path: &Path, total: u64) -> std::io::Result<()> {
    let mut done = 0u64;
    remove_inner(path, total, &mut done)
}

fn remove_inner(path: &Path, total: u64, done: &mut u64) -> std::io::Result<()> {
    for entry in std::fs::read_dir(path)? {
        let entry = entry?;
        let meta = entry.metadata()?;
        let p = entry.path();

        if crate::ui::is_cancelled() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Interrupted,
                "cancelled by user",
            ));
        }

        if meta.is_dir() {
            remove_inner(&p, total, done)?;
            let _ = std::fs::remove_dir(&p);
        } else {
            let sz = meta.len();
            std::fs::remove_file(&p)?;
            *done = done.saturating_add(sz);
            if total > 0 {
                crate::ui::progress(*done, total);
            }
        }
    }
    Ok(())
}