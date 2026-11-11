//! The `layoutsmart` command line: arguments in, text and an exit code out.

use crate::displays::same_display_set;
use crate::matcher::Move;
use crate::model::DisplayInfo;
use crate::service::{self, Backend, Error, Restored, Saved};
use crate::windows::LiveWindow;
use crate::{names, timefmt};
use std::io::Write;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const VERSION_LINE: &str = concat!("layoutsmart ", env!("CARGO_PKG_VERSION"));

pub const USAGE: &str = "\
layoutsmart — save and restore window layouts

USAGE
  layoutsmart <name> save          Save the current window positions as <name>
  layoutsmart <name> restore       Put the windows back where <name> has them
  layoutsmart restore              Restore the profile saved with the displays connected now
  layoutsmart profiles             List saved profiles (* = saved with the current displays)
  layoutsmart rename <old> <new>   Rename a profile
  layoutsmart delete <name>        Delete a profile
  layoutsmart list                 Show open windows and their positions
  layoutsmart displays             Show connected displays
  layoutsmart help | --version

'save <name>' and 'restore <name>' work too. Profiles are JSON files in
~/.config/layoutsmart (set LAYOUTSMART_DIR to use another folder).";

#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    Print(&'static str),
    Save(String),
    Restore(Option<String>),
    List,
    Displays,
    Profiles,
    Rename(String, String),
    Delete(String),
}

/// Why a run failed, and the exit code to report (2 = bad usage, 1 = could not do it).
#[derive(Debug, Clone, PartialEq)]
pub struct Failure {
    pub code: i32,
    pub message: String,
}

impl From<Error> for Failure {
    fn from(error: Error) -> Self {
        Failure {
            code: 1,
            message: error.to_string(),
        }
    }
}

fn usage_error(args: &[String]) -> Failure {
    Failure {
        code: 2,
        message: format!(
            "unrecognised command '{}'.\nRun 'layoutsmart help' for usage.",
            args.join(" ")
        ),
    }
}

/// `<name> save` and `<name> restore` read naturally, so accept them as well as
/// `save <name>` / `restore <name>`.
fn words(args: &[String]) -> Vec<&str> {
    let mut words: Vec<&str> = args.iter().map(String::as_str).collect();
    if words.len() == 2 && !names::is_reserved(words[0]) && matches!(words[1], "save" | "restore") {
        words.swap(0, 1);
    }
    words
}

fn without_names(words: &[&str]) -> Option<Command> {
    match words {
        [] | ["help"] | ["-h"] | ["--help"] => Some(Command::Print(USAGE)),
        ["--version"] | ["-V"] => Some(Command::Print(VERSION_LINE)),
        ["list"] => Some(Command::List),
        ["displays"] => Some(Command::Displays),
        ["profiles"] => Some(Command::Profiles),
        ["restore"] => Some(Command::Restore(None)),
        _ => None,
    }
}

fn with_names(words: &[&str]) -> Option<Command> {
    match words {
        ["save", name] => Some(Command::Save(name.to_string())),
        ["restore", name] => Some(Command::Restore(Some(name.to_string()))),
        ["delete", name] => Some(Command::Delete(name.to_string())),
        ["rename", old, new] => Some(Command::Rename(old.to_string(), new.to_string())),
        _ => None,
    }
}

pub fn parse(args: &[String]) -> Result<Command, Failure> {
    let words = words(args);
    without_names(&words)
        .or_else(|| with_names(&words))
        .ok_or_else(|| usage_error(args))
}

fn plural(count: usize, noun: &str) -> String {
    if count == 1 {
        format!("1 {noun}")
    } else {
        format!("{count} {noun}s")
    }
}

fn saved_text(saved: &Saved) -> String {
    format!(
        "Saved {} to '{}'.\nDisplays: {}\nFile: {}\n",
        plural(saved.windows, "window"),
        saved.profile,
        saved.displays.join(", "),
        saved.path.display()
    )
}

fn window_line(window: &Move) -> String {
    let title = if window.title.is_empty() {
        "<untitled>"
    } else {
        &window.title
    };
    format!("  {}: {}\n", window.app, title)
}

fn restored_text(restored: &Restored) -> String {
    let mut text = format!(
        "Restored '{}': moved {}.\n",
        restored.profile,
        plural(restored.moved.len(), "window")
    );
    text.extend(restored.moved.iter().map(window_line));
    if !restored.failed.is_empty() {
        text.push_str(&format!(
            "Could not move {}:\n",
            plural(restored.failed.len(), "window")
        ));
        text.extend(restored.failed.iter().map(window_line));
    }
    text
}

fn list_text(windows: &[LiveWindow]) -> String {
    windows
        .iter()
        .map(|w| {
            format!(
                "{:<24} {:>6.0},{:<6.0} {:>5.0}x{:<5.0}  {}\n",
                w.app, w.frame.x, w.frame.y, w.frame.width, w.frame.height, w.title
            )
        })
        .collect()
}

fn displays_text(backend: &Backend) -> String {
    let displays = service::displays(backend);
    let mut text: String = displays
        .iter()
        .map(|d| {
            format!(
                "{:<28} id={:<24} origin={:.0},{:.0} size={:.0}x{:.0}\n",
                d.name, d.id, d.x, d.y, d.width, d.height
            )
        })
        .collect();
    let matching = backend.store.profile_for(&displays);
    text.push_str(&format!(
        "Matching profile: {}\n",
        matching.as_deref().unwrap_or("none")
    ));
    text
}

fn profile_line(backend: &Backend, name: &str, connected: &[DisplayInfo]) -> String {
    match backend.store.load(name) {
        Ok(layout) => {
            let mark = if same_display_set(&layout.displays, connected) {
                '*'
            } else {
                ' '
            };
            let displays: Vec<&str> = layout.displays.iter().map(|d| d.name.as_str()).collect();
            format!(
                "{mark} {name}  {}  saved {}  [{}]\n",
                plural(layout.windows.len(), "window"),
                timefmt::display(&layout.saved_at),
                displays.join(", ")
            )
        }
        Err(_) => format!("! {name}  (unreadable file)\n"),
    }
}

fn profiles_text(backend: &Backend) -> String {
    let names = backend.store.names();
    if names.is_empty() {
        return "No saved profiles. Create one with: layoutsmart <name> save\n".to_string();
    }
    let connected = service::displays(backend);
    names
        .iter()
        .map(|name| profile_line(backend, name, &connected))
        .collect()
}

fn execute(command: &Command, backend: &Backend) -> Result<String, Error> {
    match command {
        Command::Print(text) => Ok(format!("{text}\n")),
        Command::Save(name) => service::save(backend, name).map(|saved| saved_text(&saved)),
        Command::Restore(name) => {
            service::restore_or_matching(backend, name.as_deref()).map(|r| restored_text(&r))
        }
        Command::List => service::windows(backend).map(|windows| list_text(&windows)),
        Command::Displays => Ok(displays_text(backend)),
        Command::Profiles => Ok(profiles_text(backend)),
        Command::Rename(old, new) => {
            service::rename(backend, old, new).map(|new| format!("Renamed '{old}' to '{new}'.\n"))
        }
        Command::Delete(name) => {
            service::delete(backend, name).map(|()| format!("Deleted '{name}'.\n"))
        }
    }
}

/// Runs one command line. Normal output goes to `out`, problems to `err`.
pub fn run(args: &[String], backend: &Backend, out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    let result = parse(args).and_then(|command| execute(&command, backend).map_err(Failure::from));
    match result {
        Ok(text) => {
            let _ = out.write_all(text.as_bytes());
            0
        }
        Err(failure) => {
            let _ = writeln!(err, "layoutsmart: {}", failure.message);
            failure.code
        }
    }
}

#[cfg(test)]
mod tests;
