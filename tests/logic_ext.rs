// tests/logic_ext.rs
//! Additional cross-platform edge cases.

use csgo_legacy_fixer::url::{build_full_url, mirror_base};
use csgo_legacy_fixer::vdf::extract_paths;

#[test]
fn vdf_handles_crlf_line_endings() {
    // Steam writes libraryfolders.vdf with CRLF on Windows.
    let vdf = "\"path\"\t\"D:\\\\SteamLibrary\"\r\n\"path\"\t\"E:\\\\Games\"\r\n";
    let p = extract_paths(vdf);
    assert_eq!(p, vec![
        r"D:\SteamLibrary".to_string(),
        r"E:\Games".to_string(),
    ]);
}

#[test]
fn vdf_ignores_keys_other_than_path() {
    let vdf = "\"name\"\t\"Steam\"\n\"path\"\t\"D:\\\\Lib\"\n\"label\"\t\"\"\n";
    let p = extract_paths(vdf);
    assert_eq!(p, vec![r"D:\Lib".to_string()]);
}

#[test]
fn mirror_base_strips_query_string() {
    assert_eq!(
        mirror_base("https://host/api/update/manifest?platform=win32&arch=x64"),
        "https://host"
    );
}

#[test]
fn mirror_base_strips_trailing_slash() {
    assert_eq!(mirror_base("https://host/"), "https://host");
}

#[test]
fn mirror_base_leaves_unrelated_path_intact() {
    // Only the two known suffixes are stripped.
    assert_eq!(
        mirror_base("https://host/foo/bar"),
        "https://host/foo/bar"
    );
}

#[test]
fn build_full_url_relative_with_query_string() {
    let out = build_full_url(
        "https://host/api/update/manifest?platform=win32",
        "update.gcup?v=2",
    );
    assert_eq!(out, "https://host/update.gcup?v=2");
}
