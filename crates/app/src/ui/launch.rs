//! Start-up, once AppKit is ready.

use super::login::LoginService;
use super::state::App;
use super::{dashboard, main_menu, status};
use layoutsmart_core::startup;

fn executable_path() -> String {
    std::env::current_exe()
        .map(|path| path.to_string_lossy().into_owned())
        .unwrap_or_default()
}

pub fn finished(app: &'static App) {
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
