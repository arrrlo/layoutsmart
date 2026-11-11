//! Running applications that can own ordinary windows.

use objc2_app_kit::{NSApplicationActivationPolicy, NSRunningApplication, NSWorkspace};

pub struct App {
    pub pid: i32,
    pub name: Option<String>,
    pub bundle_id: Option<String>,
}

fn describe(app: &NSRunningApplication) -> App {
    App {
        pid: app.processIdentifier(),
        name: app.localizedName().map(|name| name.to_string()),
        bundle_id: app.bundleIdentifier().map(|id| id.to_string()),
    }
}

/// Apps that appear in the Dock (the ones with normal windows).
pub fn regular_apps() -> Vec<App> {
    NSWorkspace::sharedWorkspace()
        .runningApplications()
        .iter()
        .filter(|app| app.activationPolicy() == NSApplicationActivationPolicy::Regular)
        .map(|app| describe(&app))
        .collect()
}
