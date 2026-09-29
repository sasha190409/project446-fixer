// tests/logic.rs
//! Cross-platform tests for the pure-logic modules.
//! These compile and run on Linux/macOS/Windows: no Windows APIs touched.

use csgo_legacy_fixer::manifest::Manifest;

// ---------- Manifest ----------

#[test]
fn manifest_full_valid() {
    let text = "\
version=1.2.3
size=1048576
sha256=0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef
sig=oPqyVpozPQECyqANC59hEubqnQMV+RgU9Yq7Io5hh1ysZBBndeTt6jNpCPbDhMSttH8ndEA/G26HVhB7PH9LBg==
url=updates/win32/update.gcup
file=update.gcup
";
    let m = Manifest::parse(text).unwrap();
    assert_eq!(m.version.as_deref(), Some("1.2.3"));
    assert_eq!(m.size, Some(1_048_576));
    assert_eq!(
        m.sha256.as_deref(),
        Some("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef")
    );
    assert_eq!(
        m.sig.as_deref(),
        Some("oPqyVpozPQECyqANC59hEubqnQMV+RgU9Yq7Io5hh1ysZBBndeTt6jNpCPbDhMSttH8ndEA/G26HVhB7PH9LBg==")
    );
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
    let m = Manifest::parse(
        "size=1\n\
         sha256=0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\n\
         sig=x\n",
    )
    .unwrap();
    assert!(m.validate().unwrap_err().to_string().contains("url"));
}

#[test]
fn manifest_missing_size_fails_validate() {
    let m = Manifest::parse(
        "url=http://x/y\n\
         sha256=0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\n\
         sig=x\n",
    )
    .unwrap();
    assert!(m.validate().unwrap_err().to_string().contains("size"));
}

#[test]
fn manifest_missing_sha256_fails_validate() {
    let m = Manifest::parse(
        "url=http://x/y\n\
         size=1\n\
         sig=x\n",
    )
    .unwrap();
    assert!(m.validate().unwrap_err().to_string().contains("sha256"));
}

#[test]
fn manifest_missing_sig_fails_validate() {
    // sha256 must be a valid 64-hex string so the validator reaches the
    // sig check instead of bailing on sha256 first.
    let m = Manifest::parse(
        "url=http://x/y\n\
         size=1\n\
         sha256=0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\n",
    )
    .unwrap();
    assert!(m.validate().unwrap_err().to_string().contains("sig"));
}

#[test]
fn manifest_invalid_sha256_fails_validate() {
    // Wrong length / non-hex → validator must reject before sig check.
    let m = Manifest::parse(
        "url=http://x/y\n\
         size=1\n\
         sha256=not-a-real-hash\n\
         sig=x\n",
    )
    .unwrap();
    assert!(m.validate().unwrap_err().to_string().contains("sha256"));
}

#[test]
fn manifest_non_numeric_size_is_error() {
    let err = Manifest::parse("size=not-a-number\n")
        .unwrap_err()
        .to_string();
    assert!(err.contains("size is not u64"), "got: {err}");
}

#[test]
fn manifest_line_without_equals_is_error() {
    let err = Manifest::parse("this-line-has-no-equals\n")
        .unwrap_err()
        .to_string();
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
