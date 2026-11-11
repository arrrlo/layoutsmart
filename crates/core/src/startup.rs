//! What happens once, when the menu bar app starts.

use crate::flows::Session;
use crate::ports::LoginItem;
use std::fs;
use std::path::Path;

/// Left in the profile folder once the app has registered itself to start at login.
pub const LOGIN_MARKER: &str = ".login-item-registered";

/// True for a copy in `/Applications` or `~/Applications` — not for a development build.
pub fn is_installed(executable_path: &str) -> bool {
    executable_path.contains("/Applications/")
}

/// `--dashboard` opens the dashboard straight away (used for screenshots and testing).
pub fn wants_dashboard(args: &[String]) -> bool {
    args.iter().any(|arg| arg == "--dashboard")
}

/// Registers only once: if the user later switches "Open at Login" off in System
/// Settings, the app must not switch it back on.
fn register_login_item_once(folder: &Path, login: &dyn LoginItem) {
    let marker = folder.join(LOGIN_MARKER);
    if !marker.exists() && login.register() {
        let _ = fs::create_dir_all(folder).and_then(|()| fs::write(&marker, ""));
    }
}

pub fn on_launch(session: &Session, login: &dyn LoginItem, executable_path: &str) {
    // Lets macOS show its own permission dialog and list the app under Accessibility.
    session.backend.sys.is_trusted(true);
    if is_installed(executable_path) {
        register_login_item_once(session.backend.store.dir(), login);
    }
}

#[cfg(test)]
mod tests;
