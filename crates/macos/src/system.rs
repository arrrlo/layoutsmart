//! The real implementations of the core's ports.

use crate::{apps, ax, screens};
use layoutsmart_core::displays::RawDisplay;
use layoutsmart_core::geometry::Rect;
use layoutsmart_core::ports::{Clock, WindowSystem};
use layoutsmart_core::store::{ProfileStore, default_dir};
use layoutsmart_core::windows::RawWindow;
use objc2_application_services::AXUIElement;
use objc2_core_foundation::CFRetained;
use std::cell::RefCell;
use std::env;
use std::time::{SystemTime, UNIX_EPOCH};

type Element = CFRetained<AXUIElement>;

pub struct MacSystem {
    /// The windows found by the last `windows()` call; `set_frame` handles index into it.
    elements: RefCell<Vec<Element>>,
}

impl MacSystem {
    pub fn new() -> Self {
        ax::limit_waiting();
        MacSystem {
            elements: RefCell::new(Vec::new()),
        }
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

fn windows_of(app: &apps::App) -> Found {
    let windows = ax::windows_of(app.pid)
        .into_iter()
        .map(|element| {
            let raw = ax::read_window(&element, app);
            (element, raw)
        })
        .collect();
    Found(windows)
}

/// Every app answers over its own connection, and the first answer alone takes 10–25 ms,
/// so all apps are asked at the same time. Results keep the order of `apps`.
fn all_windows(apps: &[apps::App]) -> Vec<(Element, RawWindow)> {
    std::thread::scope(|scope| {
        let workers: Vec<_> = apps
            .iter()
            .map(|app| scope.spawn(move || windows_of(app)))
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
        let (elements, raw): (Vec<Element>, Vec<RawWindow>) =
            all_windows(&apps::regular_apps()).into_iter().unzip();
        self.elements.replace(elements);
        raw
    }

    fn set_frame(&self, handle: usize, frame: Rect) -> bool {
        self.elements
            .borrow()
            .get(handle)
            .is_some_and(|element| ax::set_frame(element, frame))
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
