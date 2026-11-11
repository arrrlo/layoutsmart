//! The app's conversations with the user: what to ask, in which order, and what to do
//! with the answers. The macOS layer only shows the dialogs.

use crate::matcher::Move;
use crate::names;
use crate::ports::{Question, TextPrompt, Ui};
use crate::service::{self, Backend, Error};
use crate::text::{plural, quoted};

/// Everything a conversation needs.
pub struct Session<'a> {
    pub backend: Backend<'a>,
    pub ui: &'a dyn Ui,
}

pub const SAVE_HINT: &str = "Name this arrangement — for example “home” or “office”.";
pub const RENAME_HINT: &str = "Type a new name for this profile.";
pub const REPLACE_MESSAGE: &str =
    "A profile with this name already exists. Replacing it overwrites its saved window positions.";
pub const DELETE_MESSAGE: &str =
    "Its saved window positions will be removed. This can’t be undone.";
pub const PERMISSION_MESSAGE: &str = "macOS only lets apps you approve see and move other apps’ \
     windows.\n\nOpen System Settings → Privacy & Security → Accessibility and switch on \
     Layoutsmart. If it is already on, switch it off and on again.";

/// True if windows can be controlled. Otherwise explains how to allow it.
fn ensure_trusted(session: &Session) -> bool {
    if session.backend.sys.is_trusted(false) {
        return true;
    }
    let open_settings = session.ui.confirm(&Question {
        title: "Layoutsmart needs Accessibility permission".into(),
        message: PERMISSION_MESSAGE.into(),
        confirm: "Open Settings".into(),
        destructive: false,
    });
    if open_settings {
        session.ui.open_accessibility_settings();
    }
    false
}

/// Shows `title` with the problem if `result` is an error.
fn report<T>(session: &Session, title: &str, result: Result<T, Error>) -> Option<T> {
    result
        .map_err(|error| session.ui.alert(title, &error.to_string()))
        .ok()
}

/// Asks for a profile name until the user gives one that is valid and `acceptable`,
/// or cancels. A rejected name stays in the box so it can be corrected.
fn ask_name(
    session: &Session,
    mut prompt: TextPrompt,
    acceptable: &dyn Fn(&str) -> bool,
) -> Option<String> {
    let hint = prompt.message.clone();
    loop {
        let typed = session.ui.ask_text(&prompt)?;
        match names::validate(&typed) {
            Ok(name) => {
                if acceptable(&name) {
                    return Some(name);
                }
                prompt.value = name;
            }
            Err(problem) => {
                prompt.message = format!("{problem}\n{hint}");
                prompt.value = typed;
            }
        }
    }
}

fn may_use_name(session: &Session, name: &str) -> bool {
    !session.backend.store.exists(name)
        || session.ui.confirm(&Question {
            title: format!("Replace {}?", quoted(name)),
            message: REPLACE_MESSAGE.into(),
            confirm: "Replace".into(),
            destructive: true,
        })
}

/// "Save Window Positions…": returns the name the layout was saved under.
pub fn save_flow(session: &Session) -> Option<String> {
    if !ensure_trusted(session) {
        return None;
    }
    let prompt = TextPrompt {
        title: "Save Window Positions".into(),
        message: SAVE_HINT.into(),
        value: String::new(),
        confirm: "Save".into(),
    };
    let name = ask_name(session, prompt, &|name| may_use_name(session, name))?;
    let saved = service::save(&session.backend, &name);
    report(session, "Couldn’t save the window positions", saved).map(|saved| saved.profile)
}

fn stuck_line(window: &Move) -> String {
    if window.title.is_empty() {
        window.app.clone()
    } else {
        format!("{} — {}", window.app, window.title)
    }
}

/// A click on a profile: puts its windows back. True if the profile was applied.
pub fn restore_flow(session: &Session, name: &str) -> bool {
    if !ensure_trusted(session) {
        return false;
    }
    let title = format!("Couldn’t restore {}", quoted(name));
    let Some(restored) = report(session, &title, service::restore(&session.backend, name)) else {
        return false;
    };
    if !restored.failed.is_empty() {
        let lines: Vec<String> = restored.failed.iter().map(stuck_line).collect();
        let title = format!("{} didn’t move", plural(lines.len(), "window"));
        session.ui.alert(&title, &lines.join("\n"));
    }
    true
}

/// Returns the new name if the profile was renamed.
pub fn rename_flow(session: &Session, old: &str) -> Option<String> {
    let prompt = TextPrompt {
        title: format!("Rename {}", quoted(old)),
        message: RENAME_HINT.into(),
        value: old.into(),
        confirm: "Rename".into(),
    };
    let name = ask_name(session, prompt, &|_| true)?;
    let title = format!("Couldn’t rename {}", quoted(old));
    report(
        session,
        &title,
        service::rename(&session.backend, old, &name),
    )
}

/// True if the profile was deleted.
pub fn delete_flow(session: &Session, name: &str) -> bool {
    let confirmed = session.ui.confirm(&Question {
        title: format!("Delete {}?", quoted(name)),
        message: DELETE_MESSAGE.into(),
        confirm: "Delete".into(),
        destructive: true,
    });
    let title = format!("Couldn’t delete {}", quoted(name));
    confirmed && report(session, &title, service::delete(&session.backend, name)).is_some()
}

#[cfg(test)]
mod tests;
