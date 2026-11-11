//! The boundary between build logic (tested) and the real machine (`sh.rs`).

use crate::cmd::Cmd;

pub trait Runner {
    /// Runs with inherited output. True when the command exits successfully.
    fn run(&self, cmd: &Cmd) -> bool;
    /// Runs quietly and returns what the command printed, if it succeeded.
    fn capture(&self, cmd: &Cmd) -> Option<String>;
    fn write(&self, path: &str, contents: &str) -> bool;
    /// Progress output for the person running the build.
    fn say(&self, message: &str);
}

fn run_one(runner: &dyn Runner, cmd: &Cmd) -> bool {
    runner.say(&format!("▶ {cmd}"));
    let ok = runner.run(cmd);
    if !ok {
        runner.say(&format!("✗ failed: {cmd}"));
    }
    ok
}

/// Runs the commands in order and stops at the first one that fails.
pub fn run_all(runner: &dyn Runner, cmds: &[Cmd]) -> bool {
    cmds.iter().all(|cmd| run_one(runner, cmd))
}

#[cfg(test)]
mod tests;
