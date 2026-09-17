//! Game-path discovery and validation (no CLI prompts).

use anyhow::{bail, Result};
use std::path::{Path, PathBuf};

use crate::vdf::extract_paths;
use crate::win::registry;

const APPID: &str = "4465480";
const FOLDERNAME: &str = "csgo legacy";

/// Try the saved registry path first, then auto-search Steam libraries.
/// Returns `None` if nothing valid could be found — the caller is expected
/// to fall back to the folder picker.
pub fn auto_detect() -> Option<PathBuf> {
    if let Some(saved) = registry::read_string("GamePath") {
        let p = PathBuf::from(&saved);
        if validate(&p).is_ok() {
            tracing::info!(path = %saved, "path from registry");
            return Some(p);
        }
        tracing::warn!(path = %saved, "registry path invalid, discarding");
    }
    auto_search()
}

fn auto_search() -> Option<PathBuf> {
    let steam = find_steam()?;
    tracing::info!(steam = %steam.display(), "steam found");

    for candidate in [
        steam.join("steamapps").join("libraryfolders.vdf"),
        steam.join("config").join("libraryfolders.vdf"),
    ] {
        if candidate.exists() {
            if let Some(p) = search_vdf(&candidate) {
                return Some(p);
            }
        }
    }
    check_library(&steam)
}

fn find_steam() -> Option<PathBuf> {
    if let Some(p) = registry::read_hkcu_string(r"Software\Valve\Steam", "SteamPath") {
        let pb = PathBuf::from(p);
        if pb.exists() {
            return Some(pb);
        }
    }
    for key in [r"SOFTWARE\WOW6432Node\Valve\Steam", r"SOFTWARE\Valve\Steam"] {
        if let Some(p) = registry::read_hklm_string(0, key, "InstallPath") {
            let pb = PathBuf::from(p);
            if pb.exists() {
                return Some(pb);
            }
        }
    }
    for guess in [r"C:\Program Files (x86)\Steam", r"C:\Program Files\Steam"] {
        let pb = PathBuf::from(guess);
        if pb.join("steam.exe").exists() {
            return Some(pb);
        }
    }
    None
}

fn search_vdf(vdf: &Path) -> Option<PathBuf> {
    let text = std::fs::read_to_string(vdf).ok()?;
    for library in extract_paths(&text) {
        if let Some(p) = check_library(Path::new(&library)) {
            return Some(p);
        }
    }
    None
}

fn check_library(library: &Path) -> Option<PathBuf> {
    let common = library.join("steamapps").join("common");
    if !common.exists() {
        return None;
    }
    let acf = library
        .join("steamapps")
        .join(format!("appmanifest_{}.acf", APPID));
    if let Ok(text) = std::fs::read_to_string(&acf) {
        for line in text.lines() {
            let t = line.trim();
            if let Some(rest) = t.strip_prefix("\"installdir\"") {
                let rest = rest.trim_start().trim_start_matches('"');
                if let Some(end) = rest.find('"') {
                    let full = common.join(&rest[..end]);
                    if full.exists() {
                        return Some(full);
                    }
                }
            }
        }
    }
    let fallback = common.join(FOLDERNAME);
    if fallback.exists() { Some(fallback) } else { None }
}

pub fn validate(game: &Path) -> Result<()> {
    for c in [
        game.join("csgo.exe"),
        game.join("csgo"),
        game.join("bin"),
        game.join("bin").join("launcher.dll"),
        game.join("csgo").join("bin").join("client.dll"),
    ] {
        if !c.exists() {
            bail!("missing: {}", c.display());
        }
    }
    Ok(())
}