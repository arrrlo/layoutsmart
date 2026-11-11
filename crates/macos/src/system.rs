//! The real implementations of the core's ports.

use crate::{apps, ax, input, screens, server, shortcuts, spaces};
use layoutsmart_core::desktops::RawScreen;
use layoutsmart_core::displays::RawDisplay;
use layoutsmart_core::geometry::Rect;
use layoutsmart_core::ports::{Clock, Shortcut, WindowSystem};
use layoutsmart_core::store::{ProfileStore, default_dir};
use layoutsmart_core::windows::{RawWindow, ServerWindow};
use objc2_application_services::AXUIElement;
use objc2_core_foundation::CFRetained;
use std::cell::RefCell;
use std::env;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

type Element = CFRetained<AXUIElement>;

pub struct MacSystem {
    /// Every window handed to the core since the last `windows()` call; a handle is an
    /// index into this list.
    elements: RefCell<Vec<Element>>,
}

impl MacSystem {
    pub fn new() -> Self {
        ax::limit_waiting();
        MacSystem {
            elements: RefCell::new(Vec::new()),
        }
    }

    /// Keeps the elements and gives each window its handle, continuing after those
    /// already kept.
    fn keep(&self, found: Vec<(Element, RawWindow)>) -> Vec<RawWindow> {
        let first = self.elements.borrow().len();
        let (elements, raw): (Vec<Element>, Vec<RawWindow>) = found.into_iter().unzip();
        self.elements.borrow_mut().extend(elements);
        raw.into_iter()
            .enumerate()
            .map(|(index, window)| RawWindow {
                handle: first + index,
                ..window
            })
            .collect()
    }
}

impl Default for MacSystem {
    fn default() -> Self {
        MacSystem::new()
    }
}

/// One app's windows, on their way back from a worker thread.
struct Found(Vec<(Element, RawWindow)>);

// SAFETY: an AXUIElement is an immutable, reference-counted handle (a process id plus a
// token); Core Foundation reference counting is thread-safe, so it may change threads.
unsafe impl Send for Found {}

fn described(elements: Vec<Element>, app: &apps::App) -> Found {
    let windows = elements
        .into_iter()
        .map(|element| {
            let raw = ax::read_window(&element, app);
            (element, raw)
        })
        .collect();
    Found(windows)
}

/// The windows the app lists itself.
fn listed(app: &apps::App) -> Found {
    described(ax::windows_of(app.pid), app)
}

/// Every app answers over its own connection, and the first answer alone takes 10–25 ms,
/// so all apps are asked at the same time. Results keep the order of `apps`.
fn from_all<'a>(
    apps: &'a [apps::App],
    ask: impl Fn(&'a apps::App) -> Found + Sync,
) -> Vec<(Element, RawWindow)> {
    let ask = &ask;
    std::thread::scope(|scope| {
        let workers: Vec<_> = apps
            .iter()
            .map(|app| scope.spawn(move || ask(app)))
            .collect();
        workers
            .into_iter()
            .filter_map(|worker| worker.join().ok())
            .flat_map(|found| found.0)
            .collect()
    })
}

impl WindowSystem for MacSystem {
    fn is_trusted(&self, prompt: bool) -> bool {
        ax::is_trusted(prompt)
    }

    fn displays(&self) -> Vec<RawDisplay> {
        screens::displays()
    }

    fn windows(&self) -> Vec<RawWindow> {
        self.elements.borrow_mut().clear();
        self.keep(from_all(&apps::regular_apps(), listed))
    }

    fn numbered_windows(&self, wanted: &[(i32, u32)]) -> Vec<RawWindow> {
        let apps: Vec<apps::App> = apps::regular_apps()
            .into_iter()
            .filter(|app| wanted.iter().any(|(pid, _)| *pid == app.pid))
            .collect();
        let found = from_all(&apps, |app| {
            let ids: Vec<u32> = wanted
                .iter()
                .filter(|(pid, _)| *pid == app.pid)
                .map(|(_, id)| *id)
                .collect();
            described(ax::numbered_windows(app.pid, &ids), app)
        });
        self.keep(found)
    }

    fn server_windows(&self) -> Vec<ServerWindow> {
        server::windows(&apps::regular_apps())
    }

    fn screens(&self) -> Vec<RawScreen> {
        spaces::screens()
    }

    fn set_frame(&self, handle: usize, frame: Rect) -> bool {
        self.elements
            .borrow()
            .get(handle)
            .is_some_and(|element| ax::set_frame(element, frame))
    }

    fn frame(&self, handle: usize) -> Option<Rect> {
        self.elements
            .borrow()
            .get(handle)
            .and_then(|element| ax::frame(element))
    }

    fn set_fullscreen(&self, handle: usize, on: bool) -> bool {
        self.elements
            .borrow()
            .get(handle)
            .is_some_and(|element| ax::set_fullscreen(element, on))
    }

    fn desktop_shortcuts(&self) -> [Shortcut; 2] {
        shortcuts::desktop_shortcuts()
    }

    fn press_desktop_key(&self, right: bool) {
        input::press_desktop_key(right);
    }

    fn pointer(&self) -> (f64, f64) {
        input::pointer()
    }

    fn park_pointer(&self, x: f64, y: f64) {
        input::park_pointer(x, y);
    }

    fn pause(&self, milliseconds: u64) {
        std::thread::sleep(Duration::from_millis(milliseconds));
    }

    fn front_app(&self) -> Option<i32> {
        apps::front_app()
    }

    fn activate(&self, pid: i32) {
        ax::bring_to_front(pid);
    }
}

pub struct SystemClock;

impl Clock for SystemClock {
    fn unix_seconds(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|elapsed| elapsed.as_secs())
            .unwrap_or_default()
    }
}

/// `$LAYOUTSMART_DIR`, or `~/.config/layoutsmart`.
pub fn default_store() -> ProfileStore {
    ProfileStore::new(default_dir(
        env::var("LAYOUTSMART_DIR").ok(),
        env::var("HOME").ok(),
    ))
}
