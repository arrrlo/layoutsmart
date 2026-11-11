//! The one long-lived object: everything the app owns, reachable from any callback.

use super::controller::Controller;
use super::dashboard::{self, DashboardUi};
use super::dialogs::Dialogs;
use layoutsmart_core::dashboard::Dashboard;
use layoutsmart_core::flows::Session;
use layoutsmart_core::menu::Menu;
use layoutsmart_core::ports::Host;
use layoutsmart_core::service::Backend;
use layoutsmart_core::store::ProfileStore;
use layoutsmart_macos::{MacSystem, SystemClock, default_store};
use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2_app_kit::{NSApplication, NSImage, NSStatusItem};
use std::cell::{OnceCell, RefCell};
use std::rc::Rc;

pub struct App {
    pub mtm: MainThreadMarker,
    pub sys: MacSystem,
    pub store: ProfileStore,
    pub dialogs: Dialogs,
    pub controller: Retained<Controller>,
    /// Kept so the icon stays in the menu bar.
    pub status_item: RefCell<Option<Retained<NSStatusItem>>>,
    /// The bundled app icon, registered by name for dialogs (see `launch.rs`).
    pub icon: RefCell<Option<Retained<NSImage>>>,
    /// The menu as last shown; item tags index into it.
    pub menu: RefCell<Rc<Menu>>,
    pub model: RefCell<Dashboard>,
    /// The dashboard's views while its window is open.
    pub dashboard: RefCell<Option<DashboardUi>>,
}

impl App {
    pub fn new(mtm: MainThreadMarker, controller: Retained<Controller>) -> App {
        let store = default_store();
        App {
            mtm,
            sys: MacSystem::new(),
            dialogs: Dialogs::new(mtm),
            controller,
            status_item: RefCell::new(None),
            icon: RefCell::new(None),
            menu: RefCell::new(Rc::new(Menu::load(&store))),
            model: RefCell::new(Dashboard::open(&store)),
            dashboard: RefCell::new(None),
            store,
        }
    }

    pub fn session(&self) -> Session<'_> {
        Session {
            backend: Backend {
                sys: &self.sys,
                store: &self.store,
                clock: &SystemClock,
            },
            ui: &self.dialogs,
        }
    }
}

impl Host for App {
    fn open_dashboard(&self) {
        dashboard::open(self);
    }

    fn quit(&self) {
        NSApplication::sharedApplication(self.mtm).terminate(None);
    }
}

thread_local! {
    static APP: OnceCell<&'static App> = const { OnceCell::new() };
}

/// Called once, before the event loop starts.
pub fn install(app: App) {
    APP.with(|cell| cell.get_or_init(|| Box::leak(Box::new(app))).mtm);
}

pub fn app() -> &'static App {
    APP.with(|cell| {
        *cell
            .get()
            .expect("the app is set up before any event arrives")
    })
}
