#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use anyhow::Result;

fn main() -> Result<()> {
    csgo_legacy_fixer::i18n::init_logging()?;

    #[cfg(windows)]
    {
        use csgo_legacy_fixer::{gui, win};
        win::elevate::ensure_elevated()?;
        gui::run().map_err(|e| anyhow::anyhow!("gui: {e}"))?;
    }

    #[cfg(not(windows))]
    {
        anyhow::bail!("this tool only runs on Windows");
    }

    Ok(())
}