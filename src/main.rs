#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use anyhow::{Context, Result};

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

#[cfg(windows)]
fn resolve_game_path() -> Result<std::path::PathBuf> {
    use csgo_legacy_fixer::args::Lang;
    use csgo_legacy_fixer::paths;
    use csgo_legacy_fixer::win::{detect_system_lang, registry};

    // 1. Try the previously-saved path, then a full Steam-library scan.
    //    `auto_detect` already consults the registry first; the subsequent
    //    `validate` is the actual fail-closed check for the required DLLs.
    if let Some(p) = paths::auto_detect() {
        if paths::validate(&p).is_ok() {
            // Refresh the registry so future runs short-circuit on step 1.
            let _ = registry::write_string("GamePath", &p.to_string_lossy());
            early_trace(&format!("auto-detected game path: {}", p.display()));
            return Ok(p);
        }
    }

    // 2. Nothing valid found — ask the user.
    let lang = registry::read_string("Language")
        .and_then(|s| match s.as_str() {
            "2" => Some(Lang::Ru),
            "1" => Some(Lang::En),
            _ => None,
        })
        .unwrap_or_else(detect_system_lang);

    let title = match lang {
        Lang::Ru => "Выберите папку CS:GO Legacy",
        Lang::En => "Select the CS:GO Legacy folder",
    };

    let picked = rfd::FileDialog::new()
        .set_title(title)
        .pick_folder()
        .ok_or_else(|| match lang {
            Lang::Ru => anyhow::anyhow!(
                "Папка игры не выбрана. Без корректного пути к CS:GO Legacy \
                 программа не может запуститься."
            ),
            Lang::En => anyhow::anyhow!(
                "Game folder was not selected. The program cannot start \
                 without a valid CS:GO Legacy path."
            ),
        })?;

    if let Err(e) = paths::validate(&picked) {
        let prefix = match lang {
            Lang::Ru => "Выбранная папка не является корректной установкой CS:GO Legacy.",
            Lang::En => "The selected folder is not a valid CS:GO Legacy installation.",
        };
        anyhow::bail!("{prefix} {e}");
    }

    let _ = registry::write_string("GamePath", &picked.to_string_lossy());
    early_trace(&format!("user picked game path: {}", picked.display()));
    Ok(picked)
}

fn real_main() -> Result<()> {
    match csgo_legacy_fixer::i18n::init_logging() {
        Ok(()) => early_trace("init_logging ok"),
        Err(e) => early_trace(&format!("init_logging failed (continuing): {e:#}")),
    }

    #[cfg(windows)]
    {
        use csgo_legacy_fixer::gui;
        use csgo_legacy_fixer::win::elevate;

        // Manifest already requests admin, but that is bypassable
        // (RunAsInvoker, Task Scheduler, GPO, non-elevated IDE launch).
        // ensure_elevated() is the safety net.
        elevate::ensure_elevated()
            .context("administrator privileges required")?;

        // Fail-closed: no GUI without a valid game path. Any error here
        // bubbles up to `main`, which shows a MessageBox and exits with
        // status 1 — no silent shutdown.
        let initial_path = resolve_game_path()
            .context("could not determine the CS:GO Legacy game folder")?;

        gui::run(initial_path).map_err(|e| anyhow::anyhow!("gui: {e}"))?;
    }

    #[cfg(not(windows))]
    {
        anyhow::bail!("this tool only runs on Windows");
    }

    Ok(())
}
