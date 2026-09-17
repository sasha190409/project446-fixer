//! Append-only hosts merge. Port of :MergeHostsFile.

use anyhow::{Context, Result};
use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::time::Duration;

pub enum MergeResult {
    SourceMissing,
    NoChange,
    Added(usize),
}

pub fn merge(target: &Path, source: &Path) -> Result<MergeResult> {
    if !source.exists() {
        return Ok(MergeResult::SourceMissing);
    }
    let src = read_lossy(source)
        .with_context(|| format!("read source: {}", source.display()))?;
    let tgt = if target.exists() {
        read_lossy(target)
            .with_context(|| format!("read target: {}", target.display()))?
    } else {
        String::new()
    };

    let seen: HashSet<String> = tgt
        .lines()
        .map(|l| l.trim().to_ascii_lowercase())
        .filter(|l| !l.is_empty())
        .collect();

    let mut new_lines: Vec<&str> = Vec::new();
    let mut added: HashSet<String> = HashSet::new();
    for l in src.lines() {
        let t = l.trim();
        if t.is_empty() { continue; }
        let tl = t.to_ascii_lowercase();
        if seen.contains(&tl) || added.contains(&tl) { continue; }
        new_lines.push(l);
        added.insert(tl);
    }

    if new_lines.is_empty() {
        return Ok(MergeResult::NoChange);
    }

    if target.exists() {
        let ts = chrono::Local::now().format("%Y%m%d_%H%M%S");
        let bak = target.with_extension(format!("bak_{}", ts));
        let _ = fs::copy(target, &bak);
    }

    clear_readonly(target);

    let mut content = String::with_capacity(tgt.len() + 1024);
    content.push_str(&tgt);
    if !tgt.is_empty() && !tgt.ends_with('\n') {
        content.push('\n');
    }
    content.push('\n');
    content.push_str(&format!(
        "# --- added by CS:GO Legacy Fixer {} ---\n",
        chrono::Local::now().format("%Y-%m-%d %H:%M:%S")
    ));
    for l in &new_lines {
        content.push_str(l);
        content.push('\n');
    }

    write_hosts(target, content.as_bytes())
        .with_context(|| format!("write target: {}", target.display()))?;

    Ok(MergeResult::Added(new_lines.len()))
}

fn read_lossy(path: &Path) -> std::io::Result<String> {
    let bytes = fs::read(path)?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

fn clear_readonly(path: &Path) {
    if let Ok(meta) = fs::metadata(path) {
        let mut perm = meta.permissions();
        if perm.readonly() {
            perm.set_readonly(false);
            let _ = fs::set_permissions(path, perm);
        }
    }
}

/// Write `bytes` to `target`, falling back through progressively more
/// forceful strategies. The final fallback shells out to PowerShell's
/// `[System.IO.File]::WriteAllText`, which is what the original batch did —
/// its share mode (`FileShare.Read`) is compatible with the `SearchIndexer`
/// and `Dnscache` handles that hold `hosts` with `FILE_SHARE_READ`.
fn write_hosts(target: &Path, bytes: &[u8]) -> Result<()> {
    // 1. Direct write, retried on transient sharing violation.
    let mut last_err: Option<std::io::Error> = None;
    for attempt in 0..10u32 {
        match fs::write(target, bytes) {
            Ok(()) => return Ok(()),
            Err(e) if is_sharing_violation(&e) => {
                tracing::warn!(attempt, err = %e, "hosts busy, retrying");
                std::thread::sleep(Duration::from_millis(150 * (attempt as u64 + 1)));
                last_err = Some(e);
            }
            Err(e) => return Err(e).context("fs::write"),
        }
    }

    // 2. Sibling temp + atomic rename.
    match try_atomic_rename(target, bytes) {
        Ok(()) => return Ok(()),
        Err(e) => tracing::warn!(err = %e, "atomic rename failed"),
    }

    // 3. cmd copy /y (uses CopyFileEx under the hood).
    match try_cmd_copy(target, bytes) {
        Ok(()) => return Ok(()),
        Err(e) => tracing::warn!(err = %e, "cmd copy failed"),
    }

    // 4. PowerShell WriteAllText — the batch's original path.
    match try_powershell_write(target, bytes) {
        Ok(()) => return Ok(()),
        Err(e) => tracing::warn!(err = %e, "powershell write failed"),
    }

    Err(anyhow::Error::new(last_err.unwrap_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::Other, "unknown write failure")
    })))
}

fn is_sharing_violation(e: &std::io::Error) -> bool {
    e.raw_os_error() == Some(32) // ERROR_SHARING_VIOLATION
}

fn try_atomic_rename(target: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let tmp = target.with_extension(format!(
        "tmp_{}", chrono::Local::now().format("%Y%m%d_%H%M%S%3f")));
    fs::write(&tmp, bytes)?;
    let res = fs::rename(&tmp, target);
    if res.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    res
}

fn try_cmd_copy(target: &Path, bytes: &[u8]) -> Result<()> {
    let tmp = target.with_extension(format!(
        "tmp_{}", chrono::Local::now().format("%Y%m%d_%H%M%S%3f")));
    fs::write(&tmp, bytes).context("stage temp")?;
    let out = crate::win::process::hidden_command("cmd")
        .args(["/c", "copy", "/y"])
        .arg(&tmp)
        .arg(target)
        .output();
    let _ = fs::remove_file(&tmp);
    let out = out.context("spawn cmd copy")?;
    if !out.status.success() {
        anyhow::bail!("cmd copy exit={:?} stderr={}",
            out.status.code(),
            String::from_utf8_lossy(&out.stderr).trim());
    }
    Ok(())
}

fn try_powershell_write(target: &Path, bytes: &[u8]) -> Result<()> {
    let tmp = target.with_extension(format!(
        "tmp_{}", chrono::Local::now().format("%Y%m%d_%H%M%S%3f")));
    fs::write(&tmp, bytes).context("stage temp")?;

    // Let PowerShell read our temp file and copy it over the real hosts.
    // Copy-Item -Force uses CopyFileEx, which bypasses the sharing
    // restriction that plagues a plain open-for-write.
    let ps = format!(
        "$ErrorActionPreference='Stop'; \
         Copy-Item -LiteralPath '{}' -Destination '{}' -Force",
        tmp.display().to_string().replace('\'', "''"),
        target.display().to_string().replace('\'', "''"),
    );
    let out = crate::win::process::hidden_command("powershell")
        .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", &ps])
        .output();
    let _ = fs::remove_file(&tmp);
    let out = out.context("spawn powershell")?;
    if !out.status.success() {
        anyhow::bail!("powershell exit={:?} stderr={}",
            out.status.code(),
            String::from_utf8_lossy(&out.stderr).trim());
    }
    Ok(())
}