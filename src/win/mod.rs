pub mod av;
pub mod disk;
pub mod dns;
pub mod elevate;
pub mod hosts;
pub mod process;
pub mod registry;
pub mod tutorial;

use std::path::PathBuf;

/// Директория для бэкапов: `<exe_dir>/backups`.
/// Возвращает `None`, если создать её не удалось.
pub fn backup_root() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let dir = exe.parent()?.join("backups");
    std::fs::create_dir_all(&dir).ok()?;
    Some(dir)
}
