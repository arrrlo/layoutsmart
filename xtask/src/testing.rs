//! Test double for the process runner.

use crate::cmd::Cmd;
use crate::runner::Runner;
use std::cell::RefCell;

/// Records everything it is asked to do. Commands whose text contains `fail_on` fail.
#[derive(Default)]
pub struct FakeRunner {
    pub fail_on: Option<&'static str>,
    pub outputs: Vec<(&'static str, &'static str)>,
    pub log: RefCell<Vec<String>>,
    pub said: RefCell<Vec<String>>,
}

impl FakeRunner {
    pub fn failing_on(text: &'static str) -> Self {
        FakeRunner {
            fail_on: Some(text),
            ..Default::default()
        }
    }

    fn fails(&self, text: &str) -> bool {
        self.fail_on.is_some_and(|needle| text.contains(needle))
    }

    pub fn log(&self) -> Vec<String> {
        self.log.borrow().clone()
    }

    pub fn said(&self) -> String {
        self.said.borrow().join("\n")
    }
}

impl Runner for FakeRunner {
    fn run(&self, cmd: &Cmd) -> bool {
        let text = cmd.to_string();
        self.log.borrow_mut().push(text.clone());
        !self.fails(&text)
    }

    fn capture(&self, cmd: &Cmd) -> Option<String> {
        let text = cmd.to_string();
        self.log.borrow_mut().push(text.clone());
        if self.fails(&text) {
            return None;
        }
        let canned = self
            .outputs
            .iter()
            .find(|(needle, _)| text.contains(needle));
        Some(canned.map(|(_, out)| out.to_string()).unwrap_or_default())
    }

    fn write(&self, path: &str, contents: &str) -> bool {
        let text = format!("write {path}");
        self.log.borrow_mut().push(format!("{text}\n{contents}"));
        !self.fails(&text)
    }

    fn say(&self, message: &str) {
        self.said.borrow_mut().push(message.to_string());
    }
}
