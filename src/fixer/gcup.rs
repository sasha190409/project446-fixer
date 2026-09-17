//! GCUP custom archive unpacker (Rust port of gcup_extract.py).
//!
//! Format (produced by gcup_pack.py):
//!   - 4 bytes : signature "GCUP"
//!   - 1 byte  : version (0x01)
//!   - 4 bytes : file count (u32 LE) — informational, ignored on read
//!   - repeated until end:
//!       - 2 bytes : name length (u16 LE)
//!       - N bytes : name (UTF-8, may use '/' or '\')
//!       - 8 bytes : file size (u64 LE)
//!       - N bytes : file data (raw, no compression)
//!
//! The stream ends when the remaining bytes cannot be parsed as a valid
//! entry header — same fail-soft behaviour as the Python extractor.
//!
//! Security:
//!   - Path traversal (`..`), absolute components (`/`, `C:`) are rejected.
//!   - Output is always created under `output_dir`.
//!   - Cancellation is honoured between chunks.

use anyhow::{bail, Context, Result};
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};

use crate::ansi::*;
use crate::i18n::Messages;

const SIG: &[u8; 4] = b"GCUP";
const MAX_NAME_LEN: usize = 1024;
const MAX_FILE_SIZE: u64 = 4 * 1024 * 1024 * 1024;
const CHUNK: usize = 256 * 1024;

/// Extract `archive` into `output_dir`. Leaves the archive in place.
pub fn unpack(archive: &Path, output_dir: &Path, msgs: &Messages) -> Result<()> {
    println!();
    println!("{}{}{}", CYAN, msgs.gcup_unpack, RESET);

    if !archive.exists() {
        println!(
            "{}{}{}{}",
            RED,
            msgs.gcup_archive_missing,
            archive.display(),
            RESET
        );
        crate::ui::pause();
        return Ok(());
    }

    let file = std::fs::File::open(archive)
        .with_context(|| format!("open {}", archive.display()))?;
    let total = file.metadata().map(|m| m.len()).unwrap_or(0);
    let mut r = BufReader::with_capacity(1 << 20, file);

    // --- signature ---
    let mut sig = [0u8; 4];
    r.read_exact(&mut sig).context("read signature")?;
    if &sig != SIG {
        println!("{}{}{}", RED, msgs.gcup_unpack_fail, RESET);
        crate::ui::pause();
        return Ok(());
    }

    // --- version + file count (5 bytes total) ---
    let mut head = [0u8; 5];
    r.read_exact(&mut head).context("read header")?;

    std::fs::create_dir_all(output_dir)
        .with_context(|| format!("create {}", output_dir.display()))?;

    let mut pos: u64 = 9;
    crate::ui::progress(pos, total);

    let mut extracted = 0usize;
    let mut buf = vec![0u8; CHUNK];

    loop {
        crate::ui::ensure_not_cancelled()?;

        // name length
        let mut len_buf = [0u8; 2];
        match r.read_exact(&mut len_buf) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Err(e) => return Err(e).context("read name length"),
        }
        pos += 2;

        let name_len = u16::from_le_bytes(len_buf) as usize;
        if name_len == 0 || name_len > MAX_NAME_LEN {
            // End of stream or garbage — stop, matching the Python script.
            break;
        }

        // name
        let mut name_buf = vec![0u8; name_len];
        r.read_exact(&mut name_buf).context("read name")?;
        pos += name_len as u64;
        let name = String::from_utf8_lossy(&name_buf).into_owned();

        // size
        let mut size_buf = [0u8; 8];
        r.read_exact(&mut size_buf).context("read size")?;
        pos += 8;
        let size = u64::from_le_bytes(size_buf);
        if size == 0 || size > MAX_FILE_SIZE {
            break;
        }

        // Cannot be larger than what is left in the archive.
        if pos.saturating_add(size) > total {
            bail!(
                "truncated archive: {} wants {} bytes, only {} left",
                name,
                size,
                total.saturating_sub(pos)
            );
        }

        // --- path safety ---
        let dst = safe_join(output_dir, &name)?;
        if let Some(parent) = dst.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("create {}", parent.display()))?;
        }

        // --- stream body ---
        let f = std::fs::File::create(&dst)
            .with_context(|| format!("create {}", dst.display()))?;
        let mut w = BufWriter::with_capacity(1 << 20, f);

        let mut remaining = size;
        while remaining > 0 {
            crate::ui::ensure_not_cancelled()?;
            let n = remaining.min(CHUNK as u64) as usize;
            r.read_exact(&mut buf[..n]).context("read file body")?;
            w.write_all(&buf[..n]).context("write file body")?;
            remaining -= n as u64;
            pos += n as u64;
            crate::ui::progress(pos, total);
        }
        w.flush().context("flush")?;

        extracted += 1;
        println!(
            "{}{}{} ({} bytes){}",
            GRAY,
            msgs.gcup_extracted,
            name,
            size,
            RESET
        );
    }

    crate::ui::progress(0, 0);
    println!("{}{}{}", GREEN, msgs.gcup_unpack_ok, RESET);
    println!("{}{}{}{}", GRAY, msgs.gcup_files, extracted, RESET);
    Ok(())
}

/// Join a name from the archive onto `base`, rejecting anything that
/// could escape the destination directory.
///
/// Accepts both `/` and `\` as separators (Windows convention used by
/// `gcup_pack.py`). Skips empty components and `.`; rejects `..` and any
/// component containing `:`.
fn safe_join(base: &Path, name: &str) -> Result<PathBuf> {
    let mut out = PathBuf::from(base);
    let mut pushed = false;
    for part in name.split(|c| c == '/' || c == '\\') {
        if part.is_empty() || part == "." {
            continue;
        }
        if part == ".." {
            bail!("path traversal in archive: {}", name);
        }
        if part.contains(':') {
            bail!("absolute path component in archive: {}", name);
        }
        out.push(part);
        pushed = true;
    }
    if !pushed {
        bail!("empty file name in archive");
    }
    Ok(out)
}