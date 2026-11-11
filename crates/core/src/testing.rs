//! Test doubles shared by the core's unit tests.

use crate::desktops::{RawScreen, RawSpace};
use crate::displays::RawDisplay;
use crate::geometry::Rect;
use crate::ports::{Clock, Host, LoginItem, Question, TextPrompt, Ui, WindowSystem};
use crate::store::ProfileStore;
use crate::windows::{RawWindow, STANDARD_WINDOW, ServerWindow};
use std::cell::{Cell, RefCell};
use std::collections::VecDeque;

pub struct FakeSystem {
    pub trusted: Cell<bool>,
    /// How often macOS was asked to show its permission dialog.
    pub prompts: Cell<u32>,
    pub displays: Vec<RawDisplay>,
    pub windows: Vec<RawWindow>,
    /// Windows an app only gives out when addressed by number, per process id.
    pub numbered: Vec<(i32, RawWindow)>,
    /// Each by-number request: the (process id, window number) pairs asked for.
    pub scans: RefCell<Vec<Vec<(i32, u32)>>>,
    /// What the window server lists (all desktops).
    pub server: Vec<ServerWindow>,
    /// Desktops per screen; empty = macOS tells nothing.
    pub screens: Vec<RawScreen>,
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
            numbered: Vec::new(),
            scans: RefCell::new(Vec::new()),
            server: Vec::new(),
            screens: Vec::new(),
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

    /// Handles are positions in the list, whatever the test wrote in `handle`.
    fn windows(&self) -> Vec<RawWindow> {
        self.windows
            .iter()
            .enumerate()
            .map(|(handle, window)| RawWindow {
                handle,
                ..window.clone()
            })
            .collect()
    }

    /// Returns the asked-for windows it has. Handles continue after those of `windows()`.
    fn numbered_windows(&self, wanted: &[(i32, u32)]) -> Vec<RawWindow> {
        self.scans.borrow_mut().push(wanted.to_vec());
        self.numbered
            .iter()
            .filter(|(pid, window)| {
                window
                    .window_id
                    .is_some_and(|id| wanted.contains(&(*pid, id)))
            })
            .enumerate()
            .map(|(index, (_, window))| RawWindow {
                handle: self.windows.len() + index,
                ..window.clone()
            })
            .collect()
    }

    fn server_windows(&self) -> Vec<ServerWindow> {
        self.server.clone()
    }

    fn screens(&self) -> Vec<RawScreen> {
        self.screens.clone()
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
        uuid: Some(format!("uuid-{serial}")),
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
        handle: 0,
        window_id: None,
        subrole: Some(STANDARD_WINDOW.into()),
        minimized: Some(false),
        fullscreen: Some(false),
        title: Some(title.into()),
        position: Some((x, 50.0)),
        size: Some((800.0, 600.0)),
        spaces: vec![],
    }
}

/// The desktops of the screen made by `display(_, serial, _)`: `(id, is full screen)` in
/// row order, with `current` in view.
pub fn screen(serial: u32, current: u64, spaces: &[(u64, bool)]) -> RawScreen {
    RawScreen {
        uuid: format!("uuid-{serial}"),
        current,
        spaces: spaces
            .iter()
            .map(|(id, fullscreen)| RawSpace {
                id: *id,
                fullscreen: *fullscreen,
            })
            .collect(),
    }
}

/// A window the window server lists for app `app`, at `frame(x)`.
pub fn server_window(app: &str, window_id: u32, spaces: &[u64], x: f64) -> ServerWindow {
    ServerWindow {
        app: Some(app.into()),
        bundle_id: Some(format!("com.test.{}", app.to_lowercase())),
        pid: 1,
        window_id,
        layer: 0,
        alpha: 1.0,
        bounds: frame(x),
        spaces: spaces.to_vec(),
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

pub static CLOCK: FixedClock = FixedClock(NOW);

/// What the pretend user does next.
pub enum Reply {
    Text(&'static str),
    Cancel,
    Yes,
    No,
}

/// Answers dialogs from a script and records every dialog shown. When the script runs
/// out it cancels everything, so no flow can loop forever.
pub struct ScriptedUi {
    replies: RefCell<VecDeque<Reply>>,
    log: RefCell<Vec<String>>,
}

impl ScriptedUi {
    pub fn new(replies: Vec<Reply>) -> Self {
        ScriptedUi {
            replies: RefCell::new(replies.into()),
            log: RefCell::new(Vec::new()),
        }
    }

    pub fn silent() -> Self {
        ScriptedUi::new(Vec::new())
    }

    pub fn log(&self) -> Vec<String> {
        self.log.borrow().clone()
    }

    fn next(&self) -> Option<Reply> {
        self.replies.borrow_mut().pop_front()
    }
}

impl Ui for ScriptedUi {
    fn ask_text(&self, p: &TextPrompt) -> Option<String> {
        let line = format!("ask[{}|{}|{}|{}]", p.title, p.message, p.value, p.confirm);
        self.log.borrow_mut().push(line);
        match self.next() {
            Some(Reply::Text(text)) => Some(text.to_string()),
            _ => None,
        }
    }

    fn confirm(&self, q: &Question) -> bool {
        let line = format!(
            "confirm[{}|{}|{}|{}]",
            q.title, q.message, q.confirm, q.destructive
        );
        self.log.borrow_mut().push(line);
        matches!(self.next(), Some(Reply::Yes))
    }

    fn alert(&self, title: &str, message: &str) {
        self.log
            .borrow_mut()
            .push(format!("alert[{title}|{message}]"));
    }

    fn open_accessibility_settings(&self) {
        self.log.borrow_mut().push("open-settings".to_string());
    }
}

pub struct FakeLogin {
    pub accepts: bool,
    pub calls: Cell<u32>,
}

impl FakeLogin {
    pub fn new(accepts: bool) -> Self {
        FakeLogin {
            accepts,
            calls: Cell::new(0),
        }
    }
}

impl LoginItem for FakeLogin {
    fn register(&self) -> bool {
        self.calls.set(self.calls.get() + 1);
        self.accepts
    }
}

#[derive(Default)]
pub struct FakeHost {
    pub log: RefCell<Vec<&'static str>>,
}

impl Host for FakeHost {
    fn open_dashboard(&self) {
        self.log.borrow_mut().push("dashboard");
    }

    fn quit(&self) {
        self.log.borrow_mut().push("quit");
    }
}
