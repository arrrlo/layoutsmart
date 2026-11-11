//! Build tooling for Layoutsmart: quality gates and installer.
#![forbid(unsafe_code)]

pub mod cmd;
pub mod gates;
pub mod runner;
pub mod sh;

use runner::Runner;

pub const USAGE: &str = "\
Layoutsmart build tasks

  cargo xtask check      Fast gates: format, lints, tests (both architectures), coverage, CRAP
  cargo xtask mutants    Mutation tests (extra arguments go to cargo-mutants, e.g. --shard 1/4)
  cargo xtask ci         check, then mutants
  cargo xtask help       This text";

/// Runs one xtask command and returns the process exit code.
pub fn run(args: &[String], runner: &dyn Runner) -> i32 {
    let command = args.first().map_or("help", String::as_str);
    let rest = args.get(1..).unwrap_or_default();
    let ok = match command {
        "check" => gates::check(runner),
        "mutants" => gates::mutants(runner, rest),
        "ci" => gates::check(runner) && gates::mutants(runner, rest),
        _ => {
            runner.say(USAGE);
            command == "help"
        }
    };
    i32::from(!ok)
}

#[cfg(test)]
mod testing;

#[cfg(test)]
#[path = "lib_tests/tests.rs"]
mod tests;
