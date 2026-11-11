//! AppKit user interface.

mod controller;
mod dashboard;
mod dialogs;
mod launch;
mod login;
mod main_menu;
mod paint;
mod state;
mod status;

use controller::Controller;
use objc2::MainThreadMarker;
use objc2::runtime::ProtocolObject;
use objc2_app_kit::{NSApplication, NSApplicationActivationPolicy};

pub fn run() {
    let mtm = MainThreadMarker::new().expect("the app starts on the main thread");
    let application = NSApplication::sharedApplication(mtm);
    // A menu bar app: no Dock icon, no app menu.
    application.setActivationPolicy(NSApplicationActivationPolicy::Accessory);
    let controller = Controller::new(mtm);
    application.setDelegate(Some(ProtocolObject::from_ref(&*controller)));
    state::install(state::App::new(mtm, controller));
    application.run();
}
