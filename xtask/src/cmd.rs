//! A command line as data, so tests can check exactly what would be run.

use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub struct Cmd {
    pub program: String,
    pub args: Vec<String>,
    pub env: Vec<(String, String)>,
}

impl Cmd {
    /// Splits on whitespace. Use [`Cmd::arg`] for arguments that contain spaces.
    pub fn parse(line: &str) -> Cmd {
        let mut words = line.split_whitespace().map(String::from);
        Cmd {
            program: words.next().unwrap_or_default(),
            args: words.collect(),
            env: Vec::new(),
        }
    }

    pub fn arg(mut self, arg: &str) -> Cmd {
        self.args.push(arg.to_string());
        self
    }

    pub fn args(mut self, more: &[String]) -> Cmd {
        self.args.extend_from_slice(more);
        self
    }

    pub fn env(mut self, key: &str, value: &str) -> Cmd {
        self.env.push((key.to_string(), value.to_string()));
        self
    }
}

impl fmt::Display for Cmd {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let env = self.env.iter().map(|(k, v)| format!("{k}={v}"));
        let words: Vec<String> = env
            .chain([self.program.clone()])
            .chain(self.args.iter().cloned())
            .collect();
        write!(f, "{}", words.join(" "))
    }
}

#[cfg(test)]
mod tests;
