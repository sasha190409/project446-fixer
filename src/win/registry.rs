//! Thin wrapper around winreg for our own HKCU key, Steam lookups,
//! and direct environment-variable deletion.

use anyhow::{Context, Result};
use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_SET_VALUE};
use winreg::RegKey;

pub const REGROOT: &str = r"Software\CSGOLegacyFixer";

pub fn read_string(name: &str) -> Option<String> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let key = hkcu.open_subkey(REGROOT).ok()?;
    key.get_value::<String, _>(name).ok()
}

pub fn write_string(name: &str, value: &str) -> Result<()> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let (key, _) = hkcu.create_subkey(REGROOT).context("create_subkey")?;
    key.set_value(name, &value).context("set_value")?;
    Ok(())
}

pub fn read_hkcu_string(key_path: &str, name: &str) -> Option<String> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let key = hkcu.open_subkey(key_path).ok()?;
    key.get_value::<String, _>(name).ok()
}

pub fn read_hklm_string(_root: u32, key_path: &str, name: &str) -> Option<String> {
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let key = hklm.open_subkey(key_path).ok()?;
    key.get_value::<String, _>(name).ok()
}

/// Delete a value from `HKCU\Environment`.
/// Returns `Ok(true)` if it existed, `Ok(false)` if it was already absent.
pub fn delete_user_env(name: &str) -> Result<bool> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let env = hkcu
        .open_subkey_with_flags("Environment", KEY_SET_VALUE)
        .context("open HKCU\\Environment")?;

    match env.delete_value(name) {
        Ok(()) => Ok(true),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(e).context("delete env value"),
    }
}