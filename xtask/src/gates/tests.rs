use super::*;
use crate::testing::FakeRunner;

fn strings(args: &[&str]) -> Vec<String> {
    args.iter().map(|s| s.to_string()).collect()
}

#[test]
fn check_runs_every_gate_in_order() {
    let runner = FakeRunner::default();
    assert!(check(&runner));
    let exempt = EXEMPT;
    assert_eq!(
        runner.log(),
        [
            "cargo fmt --all --check".to_string(),
            "cargo clippy --workspace --all-targets -- -D warnings".to_string(),
            "cargo llvm-cov --workspace --no-report".to_string(),
            format!(
                "cargo llvm-cov report --lcov --output-path target/lcov.info --ignore-filename-regex {exempt}"
            ),
            format!(
                "cargo llvm-cov report --ignore-filename-regex {exempt} --show-missing-lines \
                 --fail-under-lines 100 --fail-under-regions 100 --fail-under-functions 100"
            ),
            "cargo crap --workspace --lcov target/lcov.info --threshold 10 --fail-above"
                .to_string(),
            "cargo test --workspace --target x86_64-apple-darwin".to_string(),
        ]
    );
}

#[test]
fn check_stops_at_the_first_failing_gate() {
    let runner = FakeRunner::failing_on("clippy");
    assert!(!check(&runner));
    assert_eq!(runner.log().len(), 2);
}

#[test]
fn exempt_pattern_names_the_shell_and_test_files() {
    for part in [
        r"/tests\.rs$",
        r"/testing\.rs$",
        "crates/macos/",
        "crates/app/src/",
        r"xtask/src/(sh|main)\.rs$",
    ] {
        assert!(EXEMPT.contains(part), "{part}");
    }
}

#[test]
fn mutants_uses_four_jobs_by_default() {
    let runner = FakeRunner::default();
    assert!(mutants(&runner, &[]));
    assert_eq!(runner.log(), ["cargo mutants --workspace -j 4"]);
}

#[test]
fn mutants_passes_extra_arguments_through() {
    let runner = FakeRunner::default();
    assert!(mutants(&runner, &strings(&["--shard", "2/4"])));
    assert_eq!(runner.log(), ["cargo mutants --workspace --shard 2/4 -j 4"]);
}

#[test]
fn mutants_respects_an_explicit_job_count() {
    for flag in ["-j", "--jobs"] {
        let runner = FakeRunner::default();
        assert!(mutants(&runner, &strings(&[flag, "2"])));
        assert_eq!(
            runner.log(),
            [format!("cargo mutants --workspace {flag} 2")]
        );
    }
}

#[test]
fn mutants_reports_failure() {
    assert!(!mutants(&FakeRunner::failing_on("mutants"), &[]));
}
