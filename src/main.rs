#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use anyhow::Result;

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

fn main() -> Result<()> {
    csgo_legacy_fixer::i18n::init_logging()?;

    #[cfg(windows)]
    {
        use csgo_legacy_fixer::gui;
        gui::run().map_err(|e| anyhow::anyhow!("gui: {e}"))?;
    }

    #[cfg(not(windows))]
    {
        anyhow::bail!("this tool only runs on Windows");
    }

    Ok(())
}
