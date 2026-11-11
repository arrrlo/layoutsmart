//! Build tooling for Layoutsmart: quality gates and installer.
#![forbid(unsafe_code)]

pub mod bundle;
pub mod cmd;
pub mod gates;
pub mod lint;
pub mod pkg;
pub mod runner;
pub mod sh;
pub mod tasks;

use bundle::Signing;
use runner::Runner;

pub const USAGE: &str = "\
Layoutsmart build tasks

Quality gates
  cargo xtask check          Format, lints, tests (both architectures), coverage, CRAP, lint-shell
  cargo xtask mutants        Mutation tests (extra arguments go to cargo-mutants, e.g. --shard 1/4)
  cargo xtask ci             check, then mutants
  cargo xtask lint-shell     Methods inside define_class! must not contain decisions

Building
  cargo xtask bundle         dist/Layoutsmart.app (universal, signed, self-contained)
  cargo xtask pkg            dist/Layoutsmart-<version>.pkg, the one-file installer
  cargo xtask release-check <tag>   The tag must be v<version>

On a desktop
  cargo xtask smoke          Drive the real app through its menu (needs Accessibility)
  cargo xtask screenshot     Redraw docs/dashboard.png from the demo profiles

  cargo xtask help           This text

Signing uses LAYOUTSMART_SIGN_IDENTITY (default: ad-hoc) and LAYOUTSMART_SIGN_KEYCHAIN.";

fn gate(command: &str, rest: &[String], runner: &dyn Runner) -> Option<bool> {
    match command {
        "check" => Some(gates::check(runner)),
        "mutants" => Some(gates::mutants(runner, rest)),
        "ci" => Some(gates::check(runner) && gates::mutants(runner, rest)),
        "lint-shell" => Some(gates::lint_shell(runner)),
        _ => None,
    }
}

fn task(command: &str, rest: &[String], runner: &dyn Runner, signing: &Signing) -> Option<bool> {
    match command {
        "bundle" => Some(bundle::bundle(runner, signing)),
        "pkg" => Some(pkg::pkg(runner, signing)),
        "release-check" => Some(tasks::release_check(
            runner,
            rest.first().map(String::as_str),
        )),
        "smoke" => Some(tasks::smoke(runner)),
        "screenshot" => Some(tasks::screenshot(runner)),
        _ => None,
    }
}

/// Runs one xtask command and returns the process exit code.
pub fn run_signed(args: &[String], runner: &dyn Runner, signing: &Signing) -> i32 {
    let command = args.first().map_or("help", String::as_str);
    let rest = args.get(1..).unwrap_or_default();
    let ok = gate(command, rest, runner)
        .or_else(|| task(command, rest, runner, signing))
        .unwrap_or_else(|| {
            runner.say(USAGE);
            command == "help"
        });
    i32::from(!ok)
}

/// As [`run_signed`], with an ad-hoc signature.
pub fn run(args: &[String], runner: &dyn Runner) -> i32 {
    run_signed(args, runner, &Signing::from_env(|_| None))
}

#[cfg(test)]
mod testing;

#[cfg(test)]
#[path = "lib_tests/tests.rs"]
mod tests;
