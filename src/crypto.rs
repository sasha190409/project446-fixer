//! Ed25519 signature verification for update.gcup.
//!
//! Public key lookup order (first match wins):
//!   1. <exe_dir>/public_key.bin   — 32 raw bytes (compressed Ed25519 point)
//!   2. <exe_dir>/public_key.txt   — base64 or hex (32 bytes)
//!   3. <exe_dir>/gc_pubkey.txt    — same as above
//!   4. <exe_dir>/csgo_gc.pub      — same as above
//!   5. EMBEDDED_PUBKEY_B64 (if non-empty)
//!
//! If no key is found anywhere, `load_public_key` returns `Ok(None)` —
//! the caller decides whether to skip or fail. A key file that exists
//! but is malformed is a hard error (fail-closed).

use anyhow::{anyhow, bail, Context, Result};
use base64::Engine;
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use std::path::{Path, PathBuf};

/// Optional compile-time key. Leave empty to require a file.
/// Base64 of the 32-byte compressed Ed25519 public key.
const EMBEDDED_PUBKEY_B64: &str = "";

const KEY_FILE_CANDIDATES: &[&str] = &[
    "public_key.bin",
    "public_key.txt",
    "gc_pubkey.txt",
    "csgo_gc.pub",
];

/// Returns `Ok(Some(key))` if a key was found, `Ok(None)` if none exists
/// anywhere, and `Err` if a file exists but cannot be parsed.
pub fn load_public_key() -> Result<Option<VerifyingKey>> {
    if let Some(dir) = exe_dir() {
        for name in KEY_FILE_CANDIDATES {
            let p = dir.join(name);
            if p.exists() {
                match load_key_from_file(&p) {
                    Ok(k) => {
                        tracing::info!(path = %p.display(), "loaded Ed25519 public key");
                        return Ok(Some(k));
                    }
                    Err(e) => {
                        tracing::warn!(path = %p.display(), error = %e, "bad public key file");
                        return Err(e).with_context(|| {
                            format!("public key file is malformed: {}", p.display())
                        });
                    }
                }
            }
        }
    }

    if !EMBEDDED_PUBKEY_B64.trim().is_empty() {
        let raw = base64::engine::general_purpose::STANDARD
            .decode(EMBEDDED_PUBKEY_B64.trim())
            .context("decode embedded pubkey (base64)")?;
        return Ok(Some(key_from_bytes(&raw)?));
    }

    tracing::warn!("no Ed25519 public key found — signature check will be skipped");
    Ok(None)
}

fn load_key_from_file(path: &Path) -> Result<VerifyingKey> {
    let bytes = std::fs::read(path).with_context(|| format!("read {}", path.display()))?;

    // 1. Raw 32 bytes
    if bytes.len() == 32 {
        return key_from_bytes(&bytes);
    }

    // 2. Text: base64 (std / urlsafe) or hex
    let text = String::from_utf8_lossy(&bytes);
    let t = text.trim();

    if let Ok(raw) = base64::engine::general_purpose::STANDARD.decode(t) {
        if raw.len() == 32 {
            return key_from_bytes(&raw);
        }
    }
    if let Ok(raw) = base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(t) {
        if raw.len() == 32 {
            return key_from_bytes(&raw);
        }
    }
    if t.len() == 64 {
        if let Ok(raw) = hex_decode(t) {
            return key_from_bytes(&raw);
        }
    }

    bail!("unrecognized public key format (expected 32 raw bytes, base64 or hex)")
}

fn key_from_bytes(bytes: &[u8]) -> Result<VerifyingKey> {
    let arr: [u8; 32] = bytes
        .try_into()
        .map_err(|_| anyhow!("public key must be exactly 32 bytes"))?;
    VerifyingKey::from_bytes(&arr).context("invalid Ed25519 public key point")
}

/// Verify `sig` (base64 or hex) over `data` using `key`.
pub fn verify(key: &VerifyingKey, data: &[u8], sig: &str) -> Result<()> {
    let sig_bytes = decode_signature(sig)?;
    let sig = Signature::from_slice(&sig_bytes)
        .map_err(|e| anyhow!("invalid signature encoding: {e}"))?;
    key.verify(data, &sig)
        .map_err(|_| anyhow!("Ed25519 signature verification failed"))
}

fn decode_signature(s: &str) -> Result<[u8; 64]> {
    let t = s.trim();

    if let Ok(raw) = base64::engine::general_purpose::STANDARD.decode(t) {
        if raw.len() == 64 {
            return raw.try_into().map_err(|_| anyhow!("bad sig length"));
        }
    }
    if let Ok(raw) = base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(t) {
        if raw.len() == 64 {
            return raw.try_into().map_err(|_| anyhow!("bad sig length"));
        }
    }
    if t.len() == 128 {
        if let Ok(raw) = hex_decode(t) {
            return raw.try_into().map_err(|_| anyhow!("bad sig length"));
        }
    }
    bail!("signature is neither valid base64 nor hex (expected 64 bytes)")
}

fn hex_decode(s: &str) -> Result<Vec<u8>> {
    if s.len() % 2 != 0 {
        bail!("odd hex length");
    }
    let mut out = Vec::with_capacity(s.len() / 2);
    let b = s.as_bytes();
    for i in (0..b.len()).step_by(2) {
        let hi = hex_nibble(b[i])?;
        let lo = hex_nibble(b[i + 1])?;
        out.push((hi << 4) | lo);
    }
    Ok(out)
}

fn hex_nibble(c: u8) -> Result<u8> {
    match c {
        b'0'..=b'9' => Ok(c - b'0'),
        b'a'..=b'f' => Ok(c - b'a' + 10),
        b'A'..=b'F' => Ok(c - b'A' + 10),
        _ => bail!("invalid hex digit: {}", c as char),
    }
}

fn exe_dir() -> Option<PathBuf> {
    std::env::current_exe()
        .ok()?
        .parent()
        .map(|p| p.to_path_buf())
}