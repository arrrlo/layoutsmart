//! Start-up, once AppKit is ready.

use super::login::LoginService;
use super::state::App;
use super::{dashboard, main_menu, status};
use layoutsmart_core::startup;
use objc2::AnyThread;
use objc2_app_kit::NSImage;
use objc2_foundation::{NSBundle, ns_string};

/// Registers the icon that ships inside the app under the name AppKit uses for "this
/// app's icon". Every dialog shows that icon; without this, the first dialog fetches it
/// from the system's icon service — a round trip to another process, and a dialog that
/// never appears if that service is not answering (seen on a CI machine). Note that
/// `setApplicationIconImage` is not enough: it does not change what the name resolves to.
/// (A development build has no bundled icon and keeps the default behaviour.)
fn use_bundled_icon(app: &App) {
    let icon = NSBundle::mainBundle()
        .pathForResource_ofType(Some(ns_string!("AppIcon")), Some(ns_string!("icns")))
        .and_then(|path| NSImage::initWithContentsOfFile(NSImage::alloc(), &path))
        .inspect(|icon| {
            icon.setName(Some(ns_string!("NSApplicationIcon")));
        });
    // Kept for the life of the app, so the name stays registered.
    app.icon.replace(icon);
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
