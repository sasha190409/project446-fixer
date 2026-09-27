#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use anyhow::Result;

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

/// Запись в фиксированный путь до всего остального.
/// `%TEMP%` есть всегда, даже когда рядом с EXE писать нельзя.
fn early_trace(msg: &str) {
    let path = std::env::temp_dir().join("csgo_legacy_fixer_startup.log");
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
    {
        use std::io::Write;
        let _ = writeln!(
            f,
            "[{}] {}",
            chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
            msg
        );
    }
}

fn install_panic_hook() {
    std::panic::set_hook(Box::new(|info| {
        let path = std::env::temp_dir().join("csgo_legacy_fixer_crash.log");
        let bt = std::backtrace::Backtrace::force_capture();
        let _ = std::fs::write(
            &path,
            format!("{}\n--- backtrace ---\n{:?}\n", info, bt),
        );
    }));
}

#[cfg(windows)]
fn show_fatal(msg: &str) {
    use windows::core::PCWSTR;
    use windows::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONERROR};
    let wide: Vec<u16> = msg.encode_utf16().chain(std::iter::once(0)).collect();
    let title: Vec<u16> = "CS:GO Legacy Fixer"
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();
    unsafe {
        let _ = MessageBoxW(
            None,
            PCWSTR(wide.as_ptr()),
            PCWSTR(title.as_ptr()),
            MB_ICONERROR,
        );
    }
}

#[cfg(not(windows))]
fn show_fatal(msg: &str) {
    eprintln!("{msg}");
}

fn main() {
    install_panic_hook();
    early_trace("=== process start ===");
    if let Ok(exe) = std::env::current_exe() {
        early_trace(&format!("exe: {}", exe.display()));
    }
    if let Ok(cwd) = std::env::current_dir() {
        early_trace(&format!("cwd: {}", cwd.display()));
    }

    // Диагностический режим: окно консоли поверх GUI.
    #[cfg(windows)]
    if std::env::args().any(|a| a == "--console") {
        unsafe {
            let _ = windows::Win32::System::Console::AllocConsole();
        }
    }

    if let Err(e) = real_main() {
        let msg = format!("CS:GO Legacy Fixer failed to start:\n\n{e:#}");
        early_trace(&format!("fatal: {e:#}"));
        show_fatal(&msg);
        std::process::exit(1);
    }
}

fn real_main() -> Result<()> {
    // Логирование best-effort: сбой не должен ронять приложение.
    match csgo_legacy_fixer::i18n::init_logging() {
        Ok(()) => early_trace("init_logging ok"),
        Err(e) => early_trace(&format!("init_logging failed (continuing): {e:#}")),
    }

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
