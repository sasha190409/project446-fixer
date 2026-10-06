//! Fix 1: stuck update. Port of :FixUpdate.

use anyhow::{bail, Context, Result};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;
use sha2::{Digest, Sha256};

use crate::ansi::*;
use crate::i18n::Messages;
use crate::manifest::Manifest;
use crate::url::build_full_url;
use crate::win::{backup_root, disk, process};

const MIRRORS: &[&str] = &[
    "https://gc.project446.com/api/update/manifest?platform=win32",
    "https://raw.githubusercontent.com/SMorganYu/csgo_gc_public/refs/heads/main/updates/win32/manifest.txt",
    "https://gitverse.ru/api/repos/smorganyu/csgo_gc_public/raw/branch/main/updates%2Fwin32%2Fmanifest.txt",
];

const USER_AGENT: &str = concat!("CSGOLegacyFixer/", env!("CARGO_PKG_VERSION"));

/// Таймаут на установку TCP-соединения (пункт 12).
const CONNECT_TIMEOUT: Duration = Duration::from_secs(30);
/// Таймаут на паузу между чтениями (сервер молчит). Отдельно от
/// общего таймаута — глобальный 900с не ловит «соединение открыто, данных нет».
const READ_TIMEOUT: Duration = Duration::from_secs(60);
/// Общий потолок на весь запрос (защита от бесконечной медленной закачки).
const TOTAL_TIMEOUT: Duration = Duration::from_secs(900);

/// Сколько дней хранить бэкапы. (пункт 11, применяется в main.rs)
pub const BACKUP_MAX_AGE_DAYS: u64 = 30;
/// Сколько самых свежих бэкапов каждого вида оставлять независимо от возраста.
pub const BACKUP_KEEP_NEWEST: usize = 5;

/// ureq 3's default TLS provider is `Rustls` even when the `rustls` cargo
/// feature is disabled — the crate doesn't auto-switch to whatever backend
/// was compiled in. Every HTTPS call fails at runtime with
/// "uri scheme is https, provider is Rustls but feature is not enabled"
/// until the provider is set explicitly. We build with `native-tls`
/// (Schannel on Windows), so pin the provider to NativeTls.
fn native_tls() -> ureq::tls::TlsConfig {
    ureq::tls::TlsConfig::builder()
        .provider(ureq::tls::TlsProvider::NativeTls)
        .build()
}

macro_rules! bail_if_cancelled {
    ($msgs:expr) => {
        if crate::ui::is_cancelled() {
            println!("{}{}{}", YELLOW, $msgs.cancelled, RESET);
            crate::ui::pause();
            return Ok(());
        }
    };
}

pub fn run(game: &Path, msgs: &Messages, _yes: bool) -> Result<()> {
    process::kill_csgo_with_message(msgs);
    println!("{}{}{}", CYAN, msgs.fix_update, RESET);

    let (mirror, text) = fetch_manifest(msgs)?;
    bail_if_cancelled!(msgs);

    let manifest = Manifest::parse(&text).context("parse manifest")?;
    manifest.validate()?;

    // Пункт 14: если манифест требует новее, чем наш бинарь — стоп.
    if let Err(e) = manifest.check_fixer_version() {
        println!("{}{}{}", RED, msgs.manifest_min_version, RESET);
        println!("{}{:#}{}", GRAY, e, RESET);
        crate::ui::pause();
        return Ok(());
    }

    println!("{}{}", msgs.version, manifest.version.as_deref().unwrap_or("?"));
    println!("{}{}", msgs.size, manifest.size.unwrap_or(0));

    let need_mb = manifest.size.unwrap_or(0) / 1_048_576 + 50;
    if !disk::has_free_mb(game, need_mb) {
        println!("{}{}{}", RED, msgs.disk_low, RESET);
        crate::ui::pause();
        return Ok(());
    }

    // Пункт 2: если в корне игры уже лежит валидный update.gcup с той же
    // подписью и хэшем — не качаем. Пункт 3: всё через mmap.
    let existing = game.join("update.gcup");
    if existing.exists() && verify_existing(&existing, &manifest)? {
        println!("{}{}{}", GREEN, msgs.update_cached_ok, RESET);
    } else {
        download_and_install(game, msgs, mirror, &manifest)?;
    }

    // Чистим stale-файлы и запускаем пост-обработку.
    let gc = game.join("csgo_gc");
    for name in [".update.lock", "update_staged.txt", "update_target.txt"] {
        let p = gc.join(name);
        if p.exists() {
            if let Err(e) = std::fs::remove_file(&p) {
                tracing::warn!(path = %p.display(), error = %e,
                               "failed to remove stale update file");
            }
        }
    }
    match crate::win::registry::delete_user_env("GC_UPDATE_DISABLE") {
        Ok(true) => tracing::info!("removed GC_UPDATE_DISABLE"),
        Ok(false) => {}
        Err(e) => tracing::warn!(error = %e, "failed to remove GC_UPDATE_DISABLE"),
    }

    let rus = game.join("csgo").join("resource").join("csgo_gc_russian.txt");
    if rus.exists() {
        if let Some(bk) = backup_root() {
            let ts = chrono::Local::now().format("%Y%m%d_%H%M%S");
            let dst = bk.join(format!("csgo_gc_russian.txt.bak_{}", ts));
            if let Err(e) = std::fs::copy(&rus, &dst) {
                tracing::warn!(source = %rus.display(), destination = %dst.display(),
                               error = %e, "failed to back up csgo_gc_russian.txt");
            }
        }
        if let Err(e) = std::fs::remove_file(&rus) {
            tracing::warn!(path = %rus.display(), error = %e,
                           "failed to remove csgo_gc_russian.txt");
        }
    }

    println!();
    println!("{}{}{}", GREEN, msgs.update_done, RESET);
    Ok(())
}

/// Проверка уже имеющегося `update.gcup`: size + sha256 + подпись.
/// Возвращает `Ok(false)` при любом несовпадении (или ошибке чтения) —
/// вызывающий просто скачает заново.
fn verify_existing(path: &Path, manifest: &Manifest) -> Result<bool> {
    let Ok(meta) = std::fs::metadata(path) else { return Ok(false) };
    let Some(expected_size) = manifest.size else { return Ok(false) };
    if meta.len() != expected_size {
        return Ok(false);
    }
    let Some(expected_sha) = manifest.sha256.as_deref() else { return Ok(false) };
    let Some(sig) = manifest.sig.as_deref() else { return Ok(false) };

    // Пункт 3: mmap — страницы файла подтягиваются по мере обхода,
    // пик RSS не равен размеру файла.
    let file = std::fs::File::open(path).context("open existing update.gcup")?;
    let mmap = unsafe { memmap2::Mmap::map(&file).context("mmap update.gcup")? };

    let mut hasher = Sha256::new();
    hasher.update(&mmap[..]);
    let actual = hex(&hasher.finalize());
    if !actual.eq_ignore_ascii_case(expected_sha) {
        return Ok(false);
    }

    let Ok(Some(pk)) = crate::crypto::load_public_key() else { return Ok(false) };
    if crate::crypto::verify(&pk, &mmap[..], sig).is_err() {
        return Ok(false);
    }
    tracing::info!(path = %path.display(), "existing update.gcup verified");
    Ok(true)
}

fn download_and_install(
    game: &Path,
    msgs: &Messages,
    mirror: &str,
    manifest: &Manifest,
) -> Result<()> {
    let rel = manifest
        .url
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("manifest: missing 'url'"))?;
    let full_url = build_full_url(mirror, rel);
    tracing::info!(url = %full_url, "downloading update");

    let tmp_dir = std::env::temp_dir().join(format!(
        "csgo_fix_{}", chrono::Local::now().format("%Y%m%d_%H%M%S")));
    std::fs::create_dir_all(&tmp_dir).context("create temp dir")?;
    let gcup_tmp = tmp_dir.join("update.gcup");

    println!();
    println!("{}{}{}", CYAN, msgs.downloading, RESET);

    let dl_result = download_to_file(&full_url, &gcup_tmp).or_else(|_| {
        if crate::ui::is_cancelled() {
            return Err(anyhow::anyhow!("cancelled"));
        }
        let base = crate::url::mirror_base(mirror);
        let alt = format!(
            "{}/{}",
            base,
            manifest.file.as_deref().unwrap_or("update.gcup")
        );
        download_to_file(&alt, &gcup_tmp)
    });

    let (actual_size, actual_hash) = match dl_result {
        Ok(v) => v,
        Err(_) => {
            if crate::ui::is_cancelled() {
                println!("{}{}{}", YELLOW, msgs.cancelled, RESET);
            } else {
                println!("{}{}{}", RED, msgs.download_fail, RESET);
                println!("{}[INFO] Temp dir kept: {}{}", GRAY, tmp_dir.display(), RESET);
            }
            crate::ui::pause();
            return Ok(());
        }
    };
    bail_if_cancelled!(msgs);

    if let Some(expected) = manifest.size {
        if actual_size != expected {
            println!("{}{}{}", RED, msgs.hash_fail, RESET);
            crate::ui::pause();
            return Ok(());
        }
    }
    if let Some(expected) = manifest.sha256.as_deref() {
        let actual = hex(&actual_hash);
        if !actual.eq_ignore_ascii_case(expected) {
            println!("{}{}{}", RED, msgs.hash_fail, RESET);
            crate::ui::pause();
            return Ok(());
        }
    }
    println!("{}{}{}", GREEN, msgs.hash_ok, RESET);

    // Ed25519 через mmap (пункт 3).
    let sig = manifest
        .sig
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("manifest: missing 'sig'"))?;

    {
        let file = std::fs::File::open(&gcup_tmp).context("open downloaded gcup")?;
        let mmap = unsafe { memmap2::Mmap::map(&file).context("mmap downloaded gcup")? };

        match crate::crypto::load_public_key() {
            Ok(Some(pk)) => {
                if let Err(e) = crate::crypto::verify(&pk, &mmap[..], sig) {
                    println!("{}{}{}", RED, msgs.sig_fail, RESET);
                    println!("{}{:#}{}", GRAY, e, RESET);
                    crate::ui::pause();
                    return Ok(());
                }
                println!("{}{}{}", GREEN, msgs.sig_verified, RESET);
            }
            Ok(None) => {
                println!("{}{}{}", RED, msgs.sig_no_key_fail, RESET);
                crate::ui::pause();
                return Ok(());
            }
            Err(e) => {
                println!("{}{}{}{}", RED, msgs.sig_bad_key, e, RESET);
                crate::ui::pause();
                return Ok(());
            }
        }
    }

    bail_if_cancelled!(msgs);

    std::fs::write(tmp_dir.join("update.gcup.sig"), sig).context("write sig")?;
    println!("{}{}{}", GREEN, msgs.sig_created, RESET);

    std::fs::copy(&gcup_tmp, game.join("update.gcup")).context("copy gcup")?;
    std::fs::copy(tmp_dir.join("update.gcup.sig"), game.join("update.gcup.sig"))
        .context("copy sig")?;
    if !game.join("update.gcup").exists() || !game.join("update.gcup.sig").exists() {
        println!("{}{}{}", RED, msgs.copy_fail, RESET);
        crate::ui::pause();
        return Ok(());
    }
    println!("{}{}{}", GREEN, msgs.copy_ok, RESET);

    let _ = std::fs::remove_dir_all(&tmp_dir);
    Ok(())
}

fn fetch_manifest(msgs: &Messages) -> Result<(&'static str, String)> {
    let mut last_err = None;
    for mirror in MIRRORS {
        match ureq::get(*mirror)
            .header("User-Agent", USER_AGENT)
            .config()
            .tls_config(native_tls())
            .timeout_connect(Some(CONNECT_TIMEOUT))
            .timeout_global(Some(TOTAL_TIMEOUT))
            .build()
            .call()
        {
            Ok(resp) => match resp.into_body().read_to_string() {
                Ok(text) if !text.trim().is_empty() => return Ok((mirror, text)),
                Ok(_) => last_err = Some(format!("{}: empty body", mirror)),
                Err(e) => last_err = Some(format!("{}: {}", mirror, e)),
            },
            Err(e) => last_err = Some(format!("{}: {}", mirror, e)),
        }
    }
    println!("{}{}{}", RED, msgs.manifest_fail, RESET);
    bail!("all mirrors failed: {}", last_err.unwrap_or_default())
}

/// Стриминговый download → файл, инкрементальный SHA256 + прогресс.
fn download_to_file(url: &str, dest: &Path) -> Result<(u64, [u8; 32])> {
    crate::ui::progress(0, 0);

    // Пункт 12: раздельные таймауты. `timeout_read` ловит «сервер
    // замолчал после установки соединения», `timeout_connect` — DNS/TCP,
    // `timeout_global` — общий потолок на случай рандомной медленной сети.
    let resp = ureq::get(url)
        .header("User-Agent", USER_AGENT)
        .config()
        .tls_config(native_tls())
        .timeout_connect(Some(CONNECT_TIMEOUT))
        .timeout_global(Some(TOTAL_TIMEOUT))
        .timeout_recv_response(Some(READ_TIMEOUT))
        .build()
        .call()
        .with_context(|| format!("GET {}", url))?;

    let total = resp
        .headers()
        .get("Content-Length")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0);

    if total > 0 {
        crate::ui::progress(0, total);
    }

    let mut reader = resp.into_body().into_reader();
    let file = std::fs::File::create(dest)
        .with_context(|| format!("create {}", dest.display()))?;
    let mut writer = std::io::BufWriter::with_capacity(1 << 20, file);
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 64 * 1024];
    let mut done = 0u64;

    loop {
        if crate::ui::is_cancelled() {
            crate::ui::progress(0, 0);
            bail!("cancelled by user");
        }
        let n = reader.read(&mut buf).context("read body")?;
        if n == 0 {
            break;
        }
        writer.write_all(&buf[..n]).context("write body")?;
        hasher.update(&buf[..n]);
        done += n as u64;
        if total > 0 {
            crate::ui::progress(done, total);
        }
    }

    writer.flush().context("flush")?;
    crate::ui::progress(0, 0);

    let hash: [u8; 32] = hasher.finalize().into();
    Ok((done, hash))
}

fn hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut s = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        s.push(HEX[(b >> 4) as usize] as char);
        s.push(HEX[(b & 0x0f) as usize] as char);
    }
    s
}
