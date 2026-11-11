use super::*;
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
    for word in ["check", "mutants", "ci", "help"] {
        assert!(USAGE.contains(&format!("cargo xtask {word}")), "{word}");
    }
}
