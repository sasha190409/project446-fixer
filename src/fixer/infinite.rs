//! Fix 3: infinite "Connecting to CS:GO network...". Port of :FixInfinite.

use anyhow::Result;
use std::path::{Path, PathBuf};

use crate::ansi::*;
use crate::i18n::Messages;
use crate::win::{av, dns, hosts, process, tutorial};

pub fn run(_game: &Path, msgs: &Messages) -> Result<()> {
    process::kill_with_message(msgs);
    check_cancel!(msgs);

    // DNS is best-effort: a failure here must not abort the whole fix.
    match dns::configure(msgs) {
        Ok(()) => {}
        Err(_) if crate::ui::is_cancelled() => {
            println!("{}{}{}", YELLOW, msgs.cancelled, RESET);
            crate::ui::pause();
            return Ok(());
        }
        Err(e) => {
            tracing::warn!(error = %e, "DNS configuration failed, continuing");
            println!("{}{:#}{}", YELLOW, e, RESET);
        }
    }
    check_cancel!(msgs);

    av::run(msgs);
    check_cancel!(msgs);

    crate::ui::pause();
    check_cancel!(msgs);

    println!();
    println!("{}{}{}", CYAN, msgs.copy_hosts, RESET);

    let exe_dir = exe_dir().unwrap_or_else(|| PathBuf::from("."));
    let target = system_hosts_path();
    let source = exe_dir.join("resources").join("hosts");

    let failed = match hosts::merge(&target, &source) {
        Ok(hosts::MergeResult::SourceMissing) => {
            println!("{}{}{}{}", RED, msgs.hosts_notfound, source.display(), RESET);
            true
        }
        Ok(hosts::MergeResult::NoChange) => {
            println!("{}{}{}{}", GREEN, msgs.hosts_nochange, target.display(), RESET);
            false
        }
        Ok(hosts::MergeResult::Added(n)) => {
            println!("{}{}{} (+{} lines){}", GREEN, msgs.hosts_ok, target.display(), n, RESET);
            false
        }
        Err(_) if crate::ui::is_cancelled() => {
            println!("{}{}{}", YELLOW, msgs.cancelled, RESET);
            crate::ui::pause();
            return Ok(());
        }
        Err(e) => {
            println!("{}{}{}{}", RED, msgs.hosts_fail, target.display(), RESET);
            println!("{}{:#}{}", GRAY, e, RESET);
            true
        }
    };
    check_cancel!(msgs);

    if failed {
        println!();
        println!("{}{}{}", RED, msgs.hosts_warn, RESET);
        crate::ui::pause();
        return Ok(());
    }

    let service = exe_dir.join("resources").join("zaprettt").join("service.bat");
    if !service.exists() {
        println!();
        println!("{}{}{}", RED, msgs.noservice, RESET);
        crate::ui::pause();
        return Ok(());
    }

    println!();
    println!("{}{}{}", YELLOW, msgs.launching, RESET);
    let _ = crate::win::process::shell_open(&service.to_string_lossy());

    // Show the tutorial as a plain-text file in Notepad instead of dumping
    // it into the log.
    let content = tutorial::text(msgs);
    let tutorial_path = std::env::temp_dir().join("csgo_legacy_fixer_tutorial.txt");
    match std::fs::write(&tutorial_path, content) {
        Ok(()) => {
            if crate::win::process::shell_open(&tutorial_path.to_string_lossy()) {
                println!("{}{}{}", GREEN, msgs.gui_tutorial_opened, RESET);
            } else {
                println!(
                    "{}{}{}{}",
                    YELLOW, msgs.gui_tutorial_open_failed, tutorial_path.display(), RESET
                );
            }
        }
        Err(_) => println!(
            "{}{}{}{}",
            YELLOW, msgs.gui_tutorial_open_failed, tutorial_path.display(), RESET
        ),
    }

    println!();
    Ok(())
}

fn exe_dir() -> Option<PathBuf> {
    Some(std::env::current_exe().ok()?.parent()?.to_path_buf())
}

fn system_hosts_path() -> PathBuf {
    let root = std::env::var_os("SystemRoot")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"C:\Windows"));
    root.join("System32").join("drivers").join("etc").join("hosts")
}