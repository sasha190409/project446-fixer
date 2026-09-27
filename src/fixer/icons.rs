//! Fix 2: broken inventory icons. Port of :FixIcons.

use anyhow::Result;
use std::path::Path;

use crate::ansi::*;
use crate::i18n::Messages;
use crate::win::{backup_root, process};

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

    let total = match
