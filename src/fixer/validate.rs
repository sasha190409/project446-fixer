//! Fix 4: kicked from server with file mismatch.
//!
//! Steps:
//!   1. Close CS:GO (Steam is left running — it hosts the validation).
//!   2. Open `steam://validate/4465480`, bring the Steam window to the
//!      foreground, then wait for the user to finish (Continue button is
//!      disabled for 10 seconds so the Steam UI can settle).
//!   3. Delete `.content_*.state` files inside `csgo_gc`.
//!   4. Run the standard update fix — this leaves `update.gcup` and
//!      `update.gcup.sig` in the game root.
//!   5. Unpack `update.gcup` in-place with the native GCUP extractor.

use anyhow::Result;
use std::path::Path;
use std::time::Duration;

use crate::ansi::*;
use crate::i18n::Messages;
use crate::win::process;

/// How long the Continue button stays disabled after launching Steam.
const CONTINUE_DELAY_SECS: u32 = 10;

pub fn run(game: &Path, msgs: &Messages) -> Result<()> {
    process::kill_csgo_with_message(msgs);
    check_cancel!(msgs);

    println!();
    println!("{}{}{}", CYAN, msgs.fix_validate, RESET);

    // Open Steam's validation page through the URL protocol handler. This
    // returns immediately and does not block on Steam exiting.
    if !process::shell_open("steam://validate/4465480") {
        tracing::warn!("ShellExecuteW(steam://validate/4465480) failed");
        println!("{}{}{}", YELLOW, msgs.validate_no_steam, RESET);
    }

    // Give Steam a moment to come up, then bring its main window to the
    // foreground. Retry for up to ~5 seconds.
    for _ in 0..20 {
        std::thread::sleep(Duration::from_millis(250));
        if crate::ui::is_cancelled() {
            break;
        }
        if process::focus_steam_window() {
            break;
        }
    }

    println!("{}{}{}", YELLOW, msgs.validate_wait, RESET);

    // Show the Continue modal, but keep the button locked for a while so
    // the user actually has time to switch to Steam and read the prompt.
    crate::ui::pause_delayed(CONTINUE_DELAY_SECS);
    check_cancel!(msgs);

    // Clean up stale content-state files in csgo_gc.
    let gc = game.join("csgo_gc");
    if !gc.exists() {
        println!("{}{}{}{}", RED, msgs.validate_no_gc, gc.display(), RESET);
    } else {
        let mut removed = 0usize;
        if let Ok(entries) = std::fs::read_dir(&gc) {
            for entry in entries.flatten() {
                let path = entry.path();
                if !path.is_file() {
                    continue;
                }
                let name = entry.file_name().to_string_lossy().into_owned();
                if name.starts_with(".content_") && name.ends_with(".state") {
                    match std::fs::remove_file(&path) {
                        Ok(()) => {
                            println!(
                                "{}{}{}{}",
                                GREEN,
                                msgs.validate_removed,
                                path.display(),
                                RESET
                            );
                            removed += 1;
                        }
                        Err(e) => {
                            println!(
                                "{}{}{}: {}{}",
                                RED,
                                msgs.validate_remove_fail,
                                path.display(),
                                e,
                                RESET
                            );
                        }
                    }
                }
            }
        }
        if removed == 0 {
            println!("{}{}{}", GRAY, msgs.validate_none, RESET);
        }
    }

    check_cancel!(msgs);

    println!();
    println!("{}{}{}", CYAN, msgs.validate_done, RESET);
    super::update::run(game, msgs, false)?;

    // update.gcup / update.gcup.sig are now in the game root; the archive
    // is intentionally left in place after extraction.
    check_cancel!(msgs);
    let gcup = game.join("update.gcup");
    super::gcup::unpack(&gcup, game, msgs)?;
    Ok(())
}
