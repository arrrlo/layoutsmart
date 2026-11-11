//! The only place that touches real processes and files. Deliberately free of decisions
//! (exempt from unit tests; see README "Contributing").

use crate::cmd::Cmd;
use crate::runner::Runner;
use std::process::Command;

pub struct Shell;

fn command(cmd: &Cmd) -> Command {
    let mut command = Command::new(&cmd.program);
    command
        .args(&cmd.args)
        .envs(cmd.env.iter().map(|(k, v)| (k, v)));
    command
}

impl Runner for Shell {
    fn run(&self, cmd: &Cmd) -> bool {
        command(cmd)
            .status()
            .map(|status| status.success())
            .unwrap_or(false)
    }

    fn capture(&self, cmd: &Cmd) -> Option<String> {
        command(cmd)
            .output()
            .ok()
            .filter(|output| output.status.success())
            .map(|output| String::from_utf8_lossy(&output.stdout).into_owned())
    }

    fn write(&self, path: &str, contents: &str) -> bool {
        std::fs::write(path, contents).is_ok()
    }

    fn say(&self, message: &str) {
        eprintln!("{message}");
    }
}
