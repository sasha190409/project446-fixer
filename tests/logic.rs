// tests/logic.rs

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
    assert_eq!(m.sig.as_deref(), Some("MEUCIQ..."));
    assert_eq!(m.url.as_deref(), Some("updates/win32/update.gcup"));
    assert_eq!(m.file.as_deref(), Some("update.gcup"));
    m.validate().unwrap();
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
