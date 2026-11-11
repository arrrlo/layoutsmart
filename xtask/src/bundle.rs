//! Builds `dist/Layoutsmart.app`: universal binaries, Info.plist, icon, signature.
//!
//! The result must run on any Mac with macOS 13 or later and nothing else installed, so
//! the build fails unless both binaries contain both architectures and link only to
//! libraries that ship with macOS.

use crate::cmd::Cmd;
use crate::runner::{Runner, run_all};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const BUNDLE_ID: &str = "io.github.arrrlo.layoutsmart";
pub const MIN_MACOS: &str = "13.0";
pub const APP: &str = "dist/Layoutsmart.app";

const TARGETS: [&str; 2] = ["aarch64-apple-darwin", "x86_64-apple-darwin"];

/// (name inside the bundle, Cargo binary it is built from). The command-line tool cannot
/// be called `layoutsmart` there: on a standard Mac disk, which ignores letter case, it
/// would collide with `Layoutsmart`.
const BINARIES: [(&str, &str); 2] = [
    ("Layoutsmart", "layoutsmart-app"),
    ("layoutsmart-cli", "layoutsmart-cli"),
];

/// How to sign. "-" is an ad-hoc signature (no certificate).
#[derive(Debug, Clone, PartialEq)]
pub struct Signing {
    pub identity: String,
    pub keychain: Option<String>,
}

impl Signing {
    /// Reads `LAYOUTSMART_SIGN_IDENTITY` and `LAYOUTSMART_SIGN_KEYCHAIN`.
    pub fn from_env(get: impl Fn(&str) -> Option<String>) -> Signing {
        let set = |key: &str| get(key).filter(|value| !value.is_empty());
        Signing {
            identity: set("LAYOUTSMART_SIGN_IDENTITY").unwrap_or_else(|| "-".into()),
            keychain: set("LAYOUTSMART_SIGN_KEYCHAIN"),
        }
    }

    fn codesign(&self) -> Cmd {
        let cmd = Cmd::parse("codesign --force --sign").arg(&self.identity);
        match &self.keychain {
            Some(keychain) => cmd.arg("--keychain").arg(keychain),
            None => cmd,
        }
    }
}

/// Replaces each `(placeholder, value)` in `template`.
pub fn render(template: &str, values: &[(&str, &str)]) -> String {
    values
        .iter()
        .fold(template.to_string(), |text, (placeholder, value)| {
            text.replace(placeholder, value)
        })
}

pub fn info_plist(version: &str) -> String {
    render(
        include_str!("../../packaging/Info.plist.in"),
        &[
            ("@VERSION@", version),
            ("@BUNDLE_ID@", BUNDLE_ID),
            ("@MIN_MACOS@", MIN_MACOS),
        ],
    )
}

/// `lipo -archs` output names both Apple Silicon and Intel.
pub fn is_universal(archs: &str) -> bool {
    let found: Vec<&str> = archs.split_whitespace().collect();
    found.contains(&"arm64") && found.contains(&"x86_64")
}

/// Libraries in `otool -L` output that are not part of macOS itself.
pub fn foreign_libraries(otool: &str) -> Vec<String> {
    otool
        .lines()
        .filter(|line| line.starts_with('\t'))
        .filter_map(|line| line.split_whitespace().next())
        .filter(|path| !path.starts_with("/System/Library/") && !path.starts_with("/usr/lib/"))
        .map(String::from)
        .collect()
}

fn inside(path: &str) -> String {
    format!("{APP}/Contents/{path}")
}

fn build_cmds() -> Vec<Cmd> {
    TARGETS
        .iter()
        .map(|target| {
            Cmd::parse("cargo build --release -p layoutsmart --target")
                .arg(target)
                .env("MACOSX_DEPLOYMENT_TARGET", MIN_MACOS)
        })
        .collect()
}

/// Merges the two single-architecture builds of one binary into a universal one.
fn lipo(name: &str, cargo_binary: &str) -> Cmd {
    let merged = Cmd::parse("lipo -create -output").arg(&inside(&format!("MacOS/{name}")));
    TARGETS.iter().fold(merged, |cmd, target| {
        cmd.arg(&format!("target/{target}/release/{cargo_binary}"))
    })
}

fn assemble_cmds() -> Vec<Cmd> {
    let mut cmds = vec![
        Cmd::parse("rm -rf").arg(APP),
        Cmd::parse("mkdir -p")
            .arg(&inside("MacOS"))
            .arg(&inside("Resources")),
    ];
    cmds.extend(BINARIES.iter().map(|(name, binary)| lipo(name, binary)));
    cmds.push(Cmd::parse("cp packaging/AppIcon.icns").arg(&inside("Resources/AppIcon.icns")));
    cmds
}

/// The helper inside the bundle first, then the bundle as a whole.
fn sign_cmds(signing: &Signing) -> Vec<Cmd> {
    vec![
        signing
            .codesign()
            .arg("--identifier")
            .arg(&format!("{BUNDLE_ID}.cli"))
            .arg(&inside("MacOS/layoutsmart-cli")),
        signing.codesign().arg(APP),
    ]
}

fn must(runner: &dyn Runner, ok: bool, problem: &str) -> bool {
    if !ok {
        runner.say(&format!("✗ {problem}"));
    }
    ok
}

fn verify_binary(runner: &dyn Runner, path: &str) -> bool {
    let archs = runner
        .capture(&Cmd::parse("lipo -archs").arg(path))
        .unwrap_or_default();
    let foreign = runner
        .capture(&Cmd::parse("otool -L").arg(path))
        .map(|output| foreign_libraries(&output))
        .unwrap_or_else(|| vec!["(could not be inspected)".into()]);
    let universal = must(
        runner,
        is_universal(&archs),
        &format!("{path} is not a universal binary (found: {})", archs.trim()),
    );
    let self_contained = must(
        runner,
        foreign.is_empty(),
        &format!(
            "{path} needs libraries that are not part of macOS: {}",
            foreign.join(", ")
        ),
    );
    universal && self_contained
}

fn verify(runner: &dyn Runner) -> bool {
    BINARIES
        .iter()
        .all(|(name, _)| verify_binary(runner, &inside(&format!("MacOS/{name}"))))
        && run_all(
            runner,
            &[Cmd::parse("codesign --verify --deep --strict").arg(APP)],
        )
}

pub fn bundle(runner: &dyn Runner, signing: &Signing) -> bool {
    run_all(runner, &build_cmds())
        && run_all(runner, &assemble_cmds())
        && runner.write(&inside("Info.plist"), &info_plist(VERSION))
        && run_all(runner, &sign_cmds(signing))
        && verify(runner)
}

#[cfg(test)]
mod tests;
