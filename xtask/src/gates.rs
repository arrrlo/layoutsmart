//! The quality gates: formatting, lints, tests on both architectures, coverage, CRAP, mutants.

use crate::cmd::Cmd;
use crate::lint;
use crate::runner::{Runner, run_all};
use std::path::PathBuf;

/// Files left out of the coverage figure: test code, and the thin macOS shell that can only
/// be exercised against a real desktop. Keep in step with `.cargo/mutants.toml`.
pub const EXEMPT: &str =
    r"(/tests\.rs$|/testing\.rs$|crates/macos/|crates/app/src/|xtask/src/(sh|main)\.rs$)";

/// The macOS shell: the two crates the lint below applies to.
pub const SHELL_DIRS: [&str; 2] = ["crates/app/src", "crates/macos/src"];

const LCOV: &str = "target/lcov.info";

fn check_cmds() -> Vec<Cmd> {
    let report = Cmd::parse("cargo llvm-cov report");
    vec![
        Cmd::parse("cargo fmt --all --check"),
        Cmd::parse("cargo clippy --workspace --all-targets -- -D warnings"),
        // Runs the native test suite once, recording coverage.
        Cmd::parse("cargo llvm-cov --workspace --no-report"),
        report
            .clone()
            .args(&["--lcov".into(), "--output-path".into(), LCOV.into()])
            .args(&["--ignore-filename-regex".into(), EXEMPT.into()]),
        report
            .args(&["--ignore-filename-regex".into(), EXEMPT.into()])
            .arg("--show-missing-lines")
            .args(&["--fail-under-lines".into(), "100".into()])
            .args(&["--fail-under-regions".into(), "100".into()])
            .args(&["--fail-under-functions".into(), "100".into()]),
        Cmd::parse("cargo crap --workspace --lcov")
            .arg(LCOV)
            .args(&["--threshold".into(), "10".into(), "--fail-above".into()]),
        // Intel build of the same tests (runs under Rosetta on Apple Silicon).
        Cmd::parse("cargo test --workspace --target x86_64-apple-darwin"),
    ]
}

/// Methods inside `define_class!` are invisible to the CRAP gate, so they may not decide
/// anything. See `lint.rs`.
pub fn lint_shell(runner: &dyn Runner) -> bool {
    let dirs: Vec<PathBuf> = SHELL_DIRS.iter().map(PathBuf::from).collect();
    lint::check(runner, &dirs)
}

/// The fast gates: everything except mutation testing.
pub fn check(runner: &dyn Runner) -> bool {
    run_all(runner, &check_cmds()) && lint_shell(runner)
}

fn sets_jobs(args: &[String]) -> bool {
    args.iter().any(|a| a == "-j" || a == "--jobs")
}

/// Mutation testing. Extra arguments (for example `--shard 1/4`) are passed through.
pub fn mutants(runner: &dyn Runner, extra: &[String]) -> bool {
    let mut cmd = Cmd::parse("cargo mutants --workspace").args(extra);
    if !sets_jobs(extra) {
        cmd = cmd.args(&["-j".into(), "4".into()]);
    }
    run_all(runner, &[cmd])
}

#[cfg(test)]
mod tests;
