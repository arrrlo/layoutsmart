//! Test doubles shared by the core's unit tests.

use crate::displays::RawDisplay;
use crate::geometry::Rect;
use crate::ports::{Clock, WindowSystem};
use crate::store::ProfileStore;
use crate::windows::{RawWindow, STANDARD_WINDOW};
use std::cell::{Cell, RefCell};

pub struct FakeSystem {
    pub trusted: Cell<bool>,
    /// How often macOS was asked to show its permission dialog.
    pub prompts: Cell<u32>,
    pub displays: Vec<RawDisplay>,
    pub windows: Vec<RawWindow>,
    /// Handles of windows that refuse to move.
    pub stuck: Vec<usize>,
    pub moves: RefCell<Vec<(usize, Rect)>>,
}

impl FakeSystem {
    pub fn new(displays: Vec<RawDisplay>, windows: Vec<RawWindow>) -> Self {
        FakeSystem {
            trusted: Cell::new(true),
            prompts: Cell::new(0),
            displays,
            windows,
            stuck: Vec::new(),
            moves: RefCell::new(Vec::new()),
        }
    }

    pub fn untrusted(self) -> Self {
        self.trusted.set(false);
        self
    }

    pub fn moves(&self) -> Vec<(usize, Rect)> {
        self.moves.borrow().clone()
    }
}

impl WindowSystem for FakeSystem {
    fn is_trusted(&self, prompt: bool) -> bool {
        if prompt && !self.trusted.get() {
            self.prompts.set(self.prompts.get() + 1);
        }
        self.trusted.get()
    }

    fn displays(&self) -> Vec<RawDisplay> {
        self.displays.clone()
    }

    fn windows(&self) -> Vec<RawWindow> {
        self.windows.clone()
    }

    fn set_frame(&self, handle: usize, frame: Rect) -> bool {
        if self.stuck.contains(&handle) {
            return false;
        }
        self.moves.borrow_mut().push((handle, frame));
        true
    }
}

pub struct FixedClock(pub u64);

impl Clock for FixedClock {
    fn unix_seconds(&self) -> u64 {
        self.0
    }
}

pub const NOW: u64 = 1_794_390_124;
pub const NOW_ISO: &str = "2026-11-11T09:42:04Z";

pub fn display(name: &str, serial: u32, x: f64) -> RawDisplay {
    RawDisplay {
        name: Some(name.into()),
        vendor: 1,
        model: 2,
        serial,
        bounds: Rect::new(x, 0.0, 1920.0, 1080.0),
    }
}

pub fn frame(x: f64) -> Rect {
    Rect::new(x, 50.0, 800.0, 600.0)
}

/// An ordinary window at `frame(x)`, owned by app `app` (bundle id `com.test.<app>`).
pub fn window(app: &str, title: &str, x: f64) -> RawWindow {
    RawWindow {
        app: Some(app.into()),
        bundle_id: Some(format!("com.test.{}", app.to_lowercase())),
        pid: 1,
        subrole: Some(STANDARD_WINDOW.into()),
        minimized: Some(false),
        fullscreen: Some(false),
        title: Some(title.into()),
        position: Some((x, 50.0)),
        size: Some((800.0, 600.0)),
    }
}

/// A laptop with one external display and three ordinary windows.
pub fn desk() -> FakeSystem {
    FakeSystem::new(
        vec![display("Built-in", 10, 0.0), display("DELL", 20, 1920.0)],
        vec![
            window("Safari", "Apple", 100.0),
            window("Ghostty", "zsh", 2000.0),
            window("Mail", "Inbox", 500.0),
        ],
    )
}

pub fn temp_store() -> (tempfile::TempDir, ProfileStore) {
    let dir = tempfile::tempdir().expect("temp dir");
    let store = ProfileStore::new(dir.path().join("profiles"));
    (dir, store)
}
