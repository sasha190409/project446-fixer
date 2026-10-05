pub mod av;
pub mod disk;
pub mod dns;
pub mod elevate;
pub mod hosts;
pub mod process;
pub mod registry;
pub mod single_instance;
pub mod tutorial;

use std::path::PathBuf;
use std::time::{Duration, SystemTime};

/// Директория для бэкапов: `<exe_dir>/backups`.
pub fn backup_root() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let dir = exe.parent()?.join("backups");
    std::fs::create_dir_all(&dir).ok()?;
    Some(dir)
}

/// Пункт 11: удаляет `econ.bak_*`, `hosts.bak_*`, `csgo_gc_russian.txt.bak_*`
/// старше `max_age`, а также всё, кроме `keep_newest` самых свежих файлов
/// каждого вида. Best-effort: ошибки логируются через `tracing::warn!`.
pub fn rotate_backups(max_age: Duration, keep_newest: usize) {
    let Some(root) = backup_root() else { return };
    let Ok(rd) = std::fs::read_dir(&root) else { return };

    // Группируем по префиксу: "econ.bak_", "hosts.bak_", ...
    let mut groups: std::collections::HashMap<String, Vec<(SystemTime, PathBuf)>> =
        std::collections::HashMap::new();

    for entry in rd.flatten() {
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|s| s.to_str()) else { continue };
        let Some(prefix) = backup_prefix(name) else { continue };
        let mtime = entry
            .metadata()
            .ok()
            .and_then(|m| m.modified().ok())
            .unwrap_or(SystemTime::UNIX_EPOCH);
        groups.entry(prefix).or_default().push((mtime, path));
    }

    let now = SystemTime::now();
    for (_prefix, mut items) in groups {
        items.sort_by(|a, b| b.0.cmp(&a.0)); // newest first
        for (i, (mtime, path)) in items.iter().enumerate() {
            let too_old = now
                .duration_since(*mtime)
                .map(|d| d > max_age)
                .unwrap_or(false);
            let beyond_keep = i >= keep_newest;
            if too_old && beyond_keep {
                if let Err(e) = std::fs::remove_dir_all(path).or_else(|_| std::fs::remove_file(path)) {
                    tracing::warn!(path = %path.display(), error = %e, "backup rotation: delete failed");
                }
            }
        }
    }
}

/// Возвращает группу бэкапа (`"econ"`, `"hosts"`, `"csgo_gc_russian.txt"`)
/// или `None`, если файл не похож на наш бэкап.
fn backup_prefix(name: &str) -> Option<String> {
    for base in ["econ", "hosts", "csgo_gc_russian.txt"] {
        let needle = format!("{}.bak_", base);
        if name.starts_with(&needle) {
            return Some(base.to_string());
        }
    }
    None
}

// ---------------------------------------------------------------------------
// System UI language
// ---------------------------------------------------------------------------

#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetUserDefaultUILanguage() -> u16;
}

pub fn detect_system_lang() -> crate::args::Lang {
    let langid = unsafe { GetUserDefaultUILanguage() };
    crate::args::Lang::from_langid(langid)
}
