//! Start-up, once AppKit is ready.

use super::login::LoginService;
use super::state::App;
use super::{dashboard, main_menu, status};
use layoutsmart_core::startup;
use objc2::AnyThread;
use objc2_app_kit::{NSApplication, NSImage};
use objc2_foundation::{NSBundle, ns_string};

/// Gives AppKit the icon that ships inside the app. Dialogs show the app icon, and
/// without this the first dialog fetches it from the system's icon service — a round trip
/// to another process, which also leaves the dialog waiting if that service is not
/// answering. (A development build has no bundled icon and keeps the default behaviour.)
fn use_bundled_icon(app: &App) {
    NSBundle::mainBundle()
        .pathForResource_ofType(Some(ns_string!("AppIcon")), Some(ns_string!("icns")))
        .and_then(|path| NSImage::initWithContentsOfFile(NSImage::alloc(), &path))
        .iter()
        // SAFETY: a plain setter, called on the main thread with a valid image.
        .for_each(|icon| unsafe {
            NSApplication::sharedApplication(app.mtm).setApplicationIconImage(Some(icon))
        });
}

fn executable_path() -> String {
    std::env::current_exe()
        .map(|path| path.to_string_lossy().into_owned())
        .unwrap_or_default()
}

pub fn finished(app: &'static App) {
    use_bundled_icon(app);
    main_menu::install(app.mtm);
    status::install(app);
    startup::on_launch(&app.session(), &LoginService, &executable_path());
    let args: Vec<String> = std::env::args().collect();
    if startup::wants_dashboard(&args) {
        dashboard::open(app);
    }
    startup::snapshot_path(&args)
        .into_iter()
        .for_each(|file| dashboard::snapshot(app, file));
}
