// tests/win_hosts.rs
#![cfg(windows)]

use csgo_legacy_fixer::win::hosts::{self, MergeResult};

#[test]
fn merge_returns_source_missing() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("hosts");
    let source = dir.path().join("does-not-exist");

    let r = hosts::merge(&target, &source).unwrap();
    assert!(matches!(r, MergeResult::SourceMissing));
}

#[test]
fn merge_returns_no_change_when_all_lines_present() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("hosts");
    let source = dir.path().join("src");
    std::fs::write(&target, "1.2.3.4 example.com\n").unwrap();
    std::fs::write(&source, "1.2.3.4 example.com\n").unwrap();

    let r = hosts::merge(&target, &source).unwrap();
    assert!(matches!(r, MergeResult::NoChange));
}

#[test]
fn merge_appends_new_lines_case_insensitively() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("hosts");
    let source = dir.path().join("src");
    std::fs::write(&target, "1.2.3.4 EXAMPLE.com\n").unwrap();
    std::fs::write(&source, "1.2.3.4 example.com\n5.6.7.8 other.net\n").unwrap();

    let r = hosts::merge(&target, &source).unwrap();
    match r {
        MergeResult::Added(n) => assert_eq!(n, 1, "only the non-duplicate line"),
        other => panic!("expected Added(1), got {:?}", matches!(other, _)),
    }

    let merged = std::fs::read_to_string(&target).unwrap();
    assert!(merged.contains("5.6.7.8 other.net"));
    // Duplicate (case-insensitive) must not be re-added.
    assert_eq!(merged.matches("example.com").count(), 1);
}

#[test]
fn merge_preserves_existing_content() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("hosts");
    let source = dir.path().join("src");
    let original = "# my hosts\n127.0.0.1 localhost\n";
    std::fs::write(&target, original).unwrap();
    std::fs::write(&source, "1.2.3.4 new.example\n").unwrap();

    hosts::merge(&target, &source).unwrap();
    let merged = std::fs::read_to_string(&target).unwrap();
    assert!(merged.starts_with(original));
    assert!(merged.contains("1.2.3.4 new.example"));
    assert!(merged.contains("added by CS:GO Legacy Fixer"));
}
