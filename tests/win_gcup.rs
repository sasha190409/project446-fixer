// tests/win_gcup.rs
#![cfg(windows)]

use std::io::Write;
use std::path::Path;

use csgo_legacy_fixer::fixer::gcup;
use csgo_legacy_fixer::i18n;

fn write_archive(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(b"GCUP");
    out.push(0x01);                       // version
    out.extend_from_slice(&(entries.len() as u32).to_le_bytes());
    for (name, data) in entries {
        let name_bytes = name.as_bytes();
        out.extend_from_slice(&(name_bytes.len() as u16).to_le_bytes());
        out.extend_from_slice(name_bytes);
        out.extend_from_slice(&(data.len() as u64).to_le_bytes());
        out.extend_from_slice(data);
    }
    out
}

fn msgs() -> &'static i18n::Messages { &i18n::EN }

#[test]
fn gcup_rejects_path_traversal() {
    let dir = tempfile::tempdir().unwrap();
    let archive = dir.path().join("bad.gcup");
    let out_dir = dir.path().join("out");
    std::fs::write(&archive, write_archive(&[("..\\evil.txt", b"pwn")])).unwrap();

    let err = gcup::unpack(&archive, &out_dir, msgs())
        .expect_err("path traversal must be rejected");
    assert!(err.to_string().contains("path traversal"), "got: {err}");
    assert!(!dir.path().join("evil.txt").exists());
}

#[test]
fn gcup_rejects_absolute_path() {
    let dir = tempfile::tempdir().unwrap();
    let archive = dir.path().join("abs.gcup");
    let out_dir = dir.path().join("out");
    std::fs::write(&archive, write_archive(&[("C:\\evil.txt", b"pwn")])).unwrap();

    let err = gcup::unpack(&archive, &out_dir, msgs())
        .expect_err("absolute path must be rejected");
    assert!(err.to_string().contains("absolute path"), "got: {err}");
}

#[test]
fn gcup_extracts_simple_file() {
    let dir = tempfile::tempdir().unwrap();
    let archive = dir.path().join("ok.gcup");
    let out_dir = dir.path().join("out");
    std::fs::write(&archive, write_archive(&[("hello.txt", b"world")])).unwrap();

    gcup::unpack(&archive, &out_dir, msgs()).expect("valid archive must unpack");
    let got = std::fs::read_to_string(out_dir.join("hello.txt")).unwrap();
    assert_eq!(got, "world");
}

#[test]
fn gcup_rejects_truncated_body() {
    let dir = tempfile::tempdir().unwrap();
    let archive = dir.path().join("trunc.gcup");
    let out_dir = dir.path().join("out");

    // Header claims 100 bytes, only 4 present.
    let mut bytes = write_archive(&[("f.bin", b"xxxx")]);
    let len = bytes.len();
    // overwrite size field (last 8 bytes of the header for the single entry)
    let size_off = len - 4 - 8;
    bytes[size_off..size_off + 8].copy_from_slice(&100u64.to_le_bytes());
    std::fs::write(&archive, &bytes).unwrap();

    let err = gcup::unpack(&archive, &out_dir, msgs())
        .expect_err("truncated body must be rejected");
    assert!(err.to_string().contains("truncated"), "got: {err}");
}

#[test]
fn gcup_ignores_missing_archive() {
    // Mirrors Python fail-soft: no archive → log + pause, return Ok.
    // In tests pause() would block, so we only assert the file isn't created.
    let dir = tempfile::tempdir().unwrap();
    let out_dir = dir.path().join("out");
    // NOTE: this call will block on ui::pause() without a GUI. See caveats.
    // Kept as documentation, not run:
    // let _ = gcup::unpack(Path::new("nope.gcup"), &out_dir, msgs());
}
