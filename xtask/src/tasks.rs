//! Smaller tasks: release check, the scripted app test, the README picture.

use crate::bundle::VERSION;
use crate::cmd::Cmd;
use crate::runner::{Runner, run_all};

const BUILD: &str = "cargo build -p layoutsmart";
const APP_BINARY: &str = "target/debug/layoutsmart-app";

/// A release is made by pushing the tag `v<version>`; anything else is a mistake.
pub fn release_check(runner: &dyn Runner, tag: Option<&str>) -> bool {
    let wanted = format!("v{VERSION}");
    let given = tag.unwrap_or_default();
    let matches = given == wanted;
    if matches {
        runner.say(&format!("✓ tag {wanted} matches the version"));
    } else {
        runner.say(&format!(
            "✗ the release tag must be {wanted} (the version in Cargo.toml), not '{given}'"
        ));
    }
    matches
}

/// Drives the real app through its menu bar icon (needs a desktop session and
/// Accessibility permission for the terminal, so it is not part of `check`).
pub fn smoke(runner: &dyn Runner) -> bool {
    run_all(
        runner,
        &[
            Cmd::parse(BUILD),
            Cmd::parse("swift scripts/smoke.swift").arg(APP_BINARY),
        ],
    )
}

/// Redraws `docs/dashboard.png` from the made-up profiles in `docs/demo-profiles`.
pub fn screenshot(runner: &dyn Runner) -> bool {
    run_all(
        runner,
        &[
            Cmd::parse(BUILD),
            Cmd::parse(APP_BINARY)
                .arg("--snapshot")
                .arg("docs/dashboard.png")
                .arg("-AppleInterfaceStyle")
                .arg("Dark")
                .env("LAYOUTSMART_DIR", "docs/demo-profiles"),
        ],
    )
}

#[cfg(test)]
mod tests;
