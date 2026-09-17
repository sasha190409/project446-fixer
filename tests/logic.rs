//! Cross-platform tests for the pure-logic modules.
//! These compile and run on Linux/macOS/Windows: no Windows APIs touched.

use csgo_legacy_fixer::manifest::Manifest;
use csgo_legacy_fixer::url::build_full_url;
use csgo_legacy_fixer::vdf::extract_paths;

// ---------- Manifest ----------

#[test]
fn manifest_full_valid() {
    let text = "\
version=1.2.3
size=1048576
sha256=ABCDEF0123456789
sig=MEUCIQ...
url=updates/win32/update.gcup
file=update.gcup
";
    let m = Manifest::parse(text).unwrap();
    assert_eq!(m.version.as_deref(), Some("1.2.3"));
    assert_eq!(m.size, Some(1_048_576));
    assert_eq!(m.sha256.as_deref(), Some("abcdef0123456789"));
    assert_eq!(m.sig.as_deref(), Some("MEUCIQ..."));
    assert_eq!(m.url.as_deref(), Some("updates/win32/update.gcup"));
    assert_eq!(m.file.as_deref(), Some("update.gcup"));
    m.validate().unwrap();
}

#[test]
fn manifest_ignores_comments_and_blank_lines() {
    let m = Manifest::parse("# comment\n\nversion=9\n   \nsize=42\n").unwrap();
    assert_eq!(m.version.as_deref(), Some("9"));
    assert_eq!(m.size, Some(42));
}

#[test]
fn manifest_trims_whitespace_around_key_and_value() {
    let m = Manifest::parse("  version   =   1.0   \n").unwrap();
    assert_eq!(m.version.as_deref(), Some("1.0"));
}

#[test]
fn manifest_unknown_keys_are_ignored() {
    let m = Manifest::parse("version=1\nfuture_key=whatever\n").unwrap();
    assert_eq!(m.version.as_deref(), Some("1"));
}

#[test]
fn manifest_missing_url_fails_validate() {
    let m = Manifest::parse("size=1\nsha256=aa\nsig=bb\n").unwrap();
    assert!(m.validate().unwrap_err().to_string().contains("url"));
}

#[test]
fn manifest_missing_size_fails_validate() {
    let m = Manifest::parse("url=http://x/y\nsha256=aa\nsig=bb\n").unwrap();
    assert!(m.validate().unwrap_err().to_string().contains("size"));
}

#[test]
fn manifest_missing_sig_fails_validate() {
    let m = Manifest::parse("url=http://x/y\nsize=1\nsha256=aa\n").unwrap();
    assert!(m.validate().unwrap_err().to_string().contains("sig"));
}

#[test]
fn manifest_non_numeric_size_is_error() {
    let err = Manifest::parse("size=not-a-number\n").unwrap_err().to_string();
    assert!(err.contains("size is not u64"), "got: {err}");
}

#[test]
fn manifest_line_without_equals_is_error() {
    let err = Manifest::parse("this-line-has-no-equals\n").unwrap_err().to_string();
    assert!(err.contains("no '='"), "got: {err}");
}

#[test]
fn manifest_equals_in_value_is_preserved() {
    let m = Manifest::parse("url=http://x/y?a=b=c\n").unwrap();
    assert_eq!(m.url.as_deref(), Some("http://x/y?a=b=c"));
}

#[test]
fn manifest_negative_size_rejected() {
    assert!(Manifest::parse("size=-1\n").is_err());
}

#[test]
fn manifest_u64_overflow_rejected() {
    assert!(Manifest::parse("size=99999999999999999999999999\n").is_err());
}

// ---------- URL ----------

#[test]
fn url_absolute_passthrough() {
    let out = build_full_url(
        "https://x/api/update/manifest",
        "https://cdn.example.com/update.gcup",
    );
    assert_eq!(out, "https://cdn.example.com/update.gcup");
}

#[test]
fn url_relative_against_api_mirror() {
    let out = build_full_url(
        "https://gc.project446.com/api/update/manifest?platform=win32",
        "updates/win32/update.gcup",
    );
    assert_eq!(out, "https://gc.project446.com/updates/win32/update.gcup");
}

#[test]
fn url_relative_against_raw_github_mirror() {
    let out = build_full_url(
        "https://raw.githubusercontent.com/SMorganYu/csgo_gc_public/refs/heads/main/updates/win32/manifest.txt",
        "update.gcup",
    );
    assert_eq!(
        out,
        "https://raw.githubusercontent.com/SMorganYu/csgo_gc_public/refs/heads/main/updates/win32/update.gcup"
    );
}

#[test]
fn url_relative_with_leading_slash_stripped() {
    let out = build_full_url("https://x/api/update/manifest", "/files/update.gcup");
    assert_eq!(out, "https://x/files/update.gcup");
}

#[test]
fn url_trailing_percent_2f_stripped() {
    let out = build_full_url(
        "https://gitverse.ru/api/repos/a/b/raw/branch/main/updates%2Fwin32%2Fmanifest.txt",
        "update.gcup",
    );
    assert_eq!(
        out,
        "https://gitverse.ru/api/repos/a/b/raw/branch/main/updates%2Fwin32/update.gcup"
    );
}

#[test]
fn url_empty_relative_produces_trailing_slash() {
    let out = build_full_url("https://x/api/update/manifest", "");
    assert_eq!(out, "https://x/");
}

// ---------- VDF ----------

#[test]
fn vdf_extracts_paths() {
    let vdf = r#"
"libraryfolders"
{
    "0"
    {
        "path"		"C:\\Program Files (x86)\\Steam"
    }
    "1"
    {
        "path"		"D:\\SteamLibrary"
    }
}
"#;
    let p = extract_paths(vdf);
    assert_eq!(p, vec![
        r"C:\Program Files (x86)\Steam".to_string(),
        r"D:\SteamLibrary".to_string(),
    ]);
}

#[test]
fn vdf_empty_input_is_empty_vec() {
    assert!(extract_paths("").is_empty());
}

#[test]
fn vdf_path_without_closing_quote_is_skipped() {
    let vdf = "\"path\"  \"D:\\Broken\n";
    assert!(extract_paths(vdf).is_empty());
}
