use super::*;
use crate::bundle::Signing;
use crate::testing::FakeRunner;

fn run_with(runner: &FakeRunner, args: &[&str]) -> i32 {
    let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    run(&args, runner)
}

#[test]
fn check_runs_the_fast_gates() {
    let runner = FakeRunner::default();
    assert_eq!(run_with(&runner, &["check"]), 0);
    assert_eq!(runner.log().len(), 7);
    assert!(!runner.log().iter().any(|c| c.contains("mutants")));
}

#[test]
fn mutants_runs_only_mutation_tests() {
    let runner = FakeRunner::default();
    assert_eq!(run_with(&runner, &["mutants", "--shard", "1/4"]), 0);
    assert_eq!(runner.log(), ["cargo mutants --workspace --shard 1/4 -j 4"]);
}

#[test]
fn lint_shell_runs_only_the_lint() {
    let runner = FakeRunner::default();
    assert_eq!(run_with(&runner, &["lint-shell"]), 0);
    assert!(runner.log().is_empty());
    assert!(runner.said().starts_with("▶ lint-shell"));
}

#[test]
fn bundle_and_pkg_use_the_signing_settings_passed_in() {
    let good = || FakeRunner {
        outputs: vec![
            ("lipo -archs", "x86_64 arm64\n"),
            ("otool -L", "bin:\n\t/usr/lib/libSystem.B.dylib (x)\n"),
        ],
        ..Default::default()
    };
    let signing = Signing {
        identity: "Someone".into(),
        keychain: None,
    };
    let args = |list: &[&str]| list.iter().map(|s| s.to_string()).collect::<Vec<_>>();

    let runner = good();
    assert_eq!(run_signed(&args(&["bundle"]), &runner, &signing), 0);
    assert_eq!(runner.log().len(), 15);
    assert!(runner.log().iter().any(|c| c.contains("--sign Someone")));

    let runner = good();
    assert_eq!(run_signed(&args(&["pkg"]), &runner, &signing), 0);
    assert_eq!(runner.log().len(), 25);

    let broken = FakeRunner::failing_on("lipo");
    assert_eq!(run_signed(&args(&["bundle"]), &broken, &signing), 1);
    assert_eq!(run_signed(&args(&["pkg"]), &broken, &signing), 1);
}

#[test]
fn the_small_tasks_are_reachable() {
    let runner = FakeRunner::default();
    assert_eq!(run_with(&runner, &["smoke"]), 0);
    assert_eq!(run_with(&runner, &["screenshot"]), 0);
    assert_eq!(runner.log().len(), 4);
    assert_eq!(run_with(&runner, &["release-check", "v0.0.0-nope"]), 1);
    assert_eq!(run_with(&runner, &["release-check"]), 1);
    let tag = format!("v{}", crate::bundle::VERSION);
    assert_eq!(run_with(&runner, &["release-check", &tag]), 0);
}

#[test]
fn ci_runs_check_then_mutants() {
    let runner = FakeRunner::default();
    assert_eq!(run_with(&runner, &["ci"]), 0);
    let log = runner.log();
    assert_eq!(log.len(), 8);
    assert_eq!(log[7], "cargo mutants --workspace -j 4");
}

#[test]
fn ci_does_not_run_mutants_when_check_fails() {
    let runner = FakeRunner::failing_on("fmt");
    assert_eq!(run_with(&runner, &["ci"]), 1);
    assert_eq!(runner.log().len(), 1);
}

#[test]
fn a_failing_gate_gives_exit_code_1() {
    assert_eq!(run_with(&FakeRunner::failing_on("clippy"), &["check"]), 1);
    assert_eq!(
        run_with(&FakeRunner::failing_on("mutants"), &["mutants"]),
        1
    );
    assert_eq!(run_with(&FakeRunner::failing_on("mutants"), &["ci"]), 1);
}

#[test]
fn help_prints_usage_and_succeeds() {
    for args in [&["help"][..], &[][..]] {
        let runner = FakeRunner::default();
        assert_eq!(run_with(&runner, args), 0);
        assert_eq!(runner.said(), USAGE);
        assert!(runner.log().is_empty());
    }
}

#[test]
fn unknown_command_prints_usage_and_fails() {
    let runner = FakeRunner::default();
    assert_eq!(run_with(&runner, &["frobnicate"]), 1);
    assert_eq!(runner.said(), USAGE);
}

#[test]
fn usage_lists_every_command() {
    for word in [
        "check",
        "mutants",
        "ci",
        "lint-shell",
        "bundle",
        "pkg",
        "smoke",
        "screenshot",
        "release-check",
        "help",
    ] {
        assert!(USAGE.contains(&format!("cargo xtask {word}")), "{word}");
    }
}
