use super::*;
use crate::bundle::VERSION;
use crate::testing::FakeRunner;

#[test]
fn release_tag_must_match_the_version() {
    let runner = FakeRunner::default();
    assert!(release_check(&runner, Some(&format!("v{VERSION}"))));
    assert_eq!(
        runner.said(),
        format!("✓ tag v{VERSION} matches the version")
    );
}

#[test]
fn a_wrong_or_missing_tag_is_refused() {
    for tag in [Some("v9.9.9"), Some(VERSION), Some(""), None] {
        let runner = FakeRunner::default();
        assert!(!release_check(&runner, tag), "{tag:?}");
        assert_eq!(
            runner.said(),
            format!(
                "✗ the release tag must be v{VERSION} (the version in Cargo.toml), not '{}'",
                tag.unwrap_or_default()
            )
        );
    }
}

#[test]
fn smoke_builds_the_app_and_drives_it() {
    let runner = FakeRunner::default();
    assert!(smoke(&runner));
    assert_eq!(
        runner.log(),
        [
            "cargo build -p layoutsmart",
            "swift scripts/smoke.swift target/debug/layoutsmart-app",
        ]
    );
    assert!(!smoke(&FakeRunner::failing_on("cargo build")));
    assert!(!smoke(&FakeRunner::failing_on("swift")));
}

#[test]
fn screenshot_draws_the_demo_profiles_in_dark_mode() {
    let runner = FakeRunner::default();
    assert!(screenshot(&runner));
    assert_eq!(
        runner.log(),
        [
            "cargo build -p layoutsmart",
            "LAYOUTSMART_DIR=docs/demo-profiles target/debug/layoutsmart-app \
             --snapshot docs/dashboard.png -AppleInterfaceStyle Dark",
        ]
    );
    assert!(!screenshot(&FakeRunner::failing_on("cargo build")));
    assert!(!screenshot(&FakeRunner::failing_on("--snapshot")));
}
