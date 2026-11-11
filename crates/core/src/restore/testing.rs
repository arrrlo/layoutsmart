//! A pretend Mac for testing save and restore: screens, each with a row of desktops, and
//! windows on them. It follows the rules macOS was measured to follow on a real machine:
//!
//! * an app lists a window only while its desktop is in view; once listed, the window can
//!   also be found by number;
//! * moving a window onto another screen puts it on that screen's desktop in view, or on
//!   the ordinary desktop last in view there if a full-screen app is in view;
//! * entering full screen adds a desktop at the end of the screen's row and shows it;
//! * leaving full screen removes that desktop. If it was in view, the screen shows the
//!   ordinary desktop last in view and the window goes there; if it was not, the screen
//!   stays as it is and the window goes back to the desktop it came from;
//! * a full-screen window cannot be moved;
//! * the "next desktop" shortcut acts on the screen under the pointer.

use crate::desktops::{RawScreen, RawSpace};
use crate::displays::RawDisplay;
use crate::geometry::Rect;
use crate::ports::{Shortcut, WindowSystem};
use crate::windows::{RawWindow, STANDARD_WINDOW, ServerWindow};
use std::cell::{Cell, RefCell};

#[derive(Clone)]
struct Space {
    id: u64,
    fullscreen: bool,
}

struct Screen {
    name: String,
    serial: u32,
    bounds: Rect,
    row: Vec<Space>,
    /// Id of the desktop in view.
    current: u64,
    /// Id of the ordinary desktop most recently in view.
    last_ordinary: u64,
}

struct Window {
    id: u32,
    pid: i32,
    app: String,
    title: String,
    frame: Rect,
    space: u64,
    fullscreen: bool,
    /// Shows on every ordinary desktop of its screen.
    everywhere: bool,
    /// Has its app ever handed it out?
    listed: bool,
    /// Its app never hands it out, in view or not.
    withheld: bool,
    /// How many more `set_frame` calls will leave the width unchanged.
    stubborn: u32,
    refuses_to_move: bool,
    cannot_fullscreen: bool,
    /// Where it returns to when it leaves full screen.
    windowed_frame: Rect,
    /// The desktop it was on when it entered full screen.
    came_from: u64,
}

/// Something that happens once enough time has passed.
#[derive(Clone, Copy)]
enum Later {
    /// A window enters or leaves full screen.
    Toggle(u32, bool),
    /// A screen (by position) shows a desktop (by id).
    Show(usize, u64),
}

/// The process id of the app the pretend user is working in.
pub const FRONT_APP: i32 = 77;

pub struct Sim {
    screens: RefCell<Vec<Screen>>,
    windows: RefCell<Vec<Window>>,
    /// Window ids by handle, as handed out since the last `windows()` call.
    handles: RefCell<Vec<u32>>,
    next_space: Cell<u64>,
    next_window: Cell<u32>,
    pointer: Cell<(f64, f64)>,
    pub front: Cell<i32>,
    pub trusted: Cell<bool>,
    /// False: the step shortcuts are switched off in the user's settings.
    pub shortcuts_on: Cell<bool>,
    /// False: pressing the shortcut has no effect.
    pub keys_work: Cell<bool>,
    /// Full-screen changes take effect only after this many `pause` calls.
    pub fullscreen_lag: Cell<u32>,
    /// A screen shows a new full-screen desktop only this many `pause` calls after the
    /// desktop has appeared (the animation is still running).
    pub switch_lag: Cell<u32>,
    pending: RefCell<Vec<(u32, Later)>>,
    /// Everything that was done to the desktop, in order.
    log: RefCell<Vec<String>>,
    /// How often every app was asked for its windows.
    pub looks: Cell<u32>,
    pub waited_ms: Cell<u64>,
}

impl Sim {
    pub fn new() -> Sim {
        Sim {
            screens: RefCell::new(Vec::new()),
            windows: RefCell::new(Vec::new()),
            handles: RefCell::new(Vec::new()),
            next_space: Cell::new(1),
            next_window: Cell::new(100),
            pointer: Cell::new((7.0, 7.0)),
            front: Cell::new(FRONT_APP),
            trusted: Cell::new(true),
            shortcuts_on: Cell::new(true),
            keys_work: Cell::new(true),
            fullscreen_lag: Cell::new(0),
            switch_lag: Cell::new(0),
            pending: RefCell::new(Vec::new()),
            log: RefCell::new(Vec::new()),
            looks: Cell::new(0),
            waited_ms: Cell::new(0),
        }
    }

    fn new_space(&self) -> u64 {
        let id = self.next_space.get();
        self.next_space.set(id + 1);
        id
    }

    /// Adds a screen with `desktops` ordinary desktops, showing number `showing` (from 1).
    pub fn screen(
        self,
        name: &str,
        serial: u32,
        bounds: Rect,
        desktops: usize,
        showing: usize,
    ) -> Sim {
        let row: Vec<Space> = (0..desktops)
            .map(|_| Space {
                id: self.new_space(),
                fullscreen: false,
            })
            .collect();
        let current = row[showing - 1].id;
        self.screens.borrow_mut().push(Screen {
            name: name.into(),
            serial,
            bounds,
            row,
            current,
            last_ordinary: current,
        });
        self
    }

    fn screen_index(&self, name: &str) -> usize {
        self.screens
            .borrow()
            .iter()
            .position(|screen| screen.name == name)
            .unwrap_or_else(|| panic!("no screen named {name}"))
    }

    fn ordinary_space(&self, screen: &str, desktop: usize) -> u64 {
        let screens = self.screens.borrow();
        let ordinary: Vec<&Space> = screens[self.screen_index(screen)]
            .row
            .iter()
            .filter(|space| !space.fullscreen)
            .collect();
        ordinary[desktop - 1].id
    }

    /// Opens a window of `app` on ordinary desktop `desktop` of `screen`. It counts as
    /// already handed out by its app (the usual case). Returns its window number.
    pub fn window(&self, app: &str, title: &str, screen: &str, desktop: usize, frame: Rect) -> u32 {
        let id = self.next_window.get();
        self.next_window.set(id + 1);
        let same_app = self
            .windows
            .borrow()
            .iter()
            .find(|window| window.app == app)
            .map(|window| window.pid);
        let pid = same_app.unwrap_or(id as i32);
        self.windows.borrow_mut().push(Window {
            id,
            pid,
            app: app.into(),
            title: title.into(),
            frame,
            space: self.ordinary_space(screen, desktop),
            fullscreen: false,
            everywhere: false,
            listed: true,
            withheld: false,
            stubborn: 0,
            refuses_to_move: false,
            cannot_fullscreen: false,
            windowed_frame: frame,
            came_from: 0,
        });
        id
    }

    fn edit(&self, id: u32, change: impl FnOnce(&mut Window)) {
        let mut windows = self.windows.borrow_mut();
        let window = windows
            .iter_mut()
            .find(|window| window.id == id)
            .unwrap_or_else(|| panic!("no window {id}"));
        change(window);
    }

    /// The window has only ever been on a desktop that was not in view.
    pub fn never_listed(&self, id: u32) {
        self.edit(id, |window| window.listed = false);
    }

    /// The window's app does not hand it out at all.
    pub fn withheld(&self, id: u32) {
        self.edit(id, |window| {
            window.listed = false;
            window.withheld = true;
        });
    }

    /// The next `passes` attempts to resize it change everything but the width.
    pub fn stubborn(&self, id: u32, passes: u32) {
        self.edit(id, |window| window.stubborn = passes);
    }

    pub fn refuses_to_move(&self, id: u32) {
        self.edit(id, |window| window.refuses_to_move = true);
    }

    /// The window's app has no full-screen mode.
    pub fn cannot_fullscreen(&self, id: u32) {
        self.edit(id, |window| window.cannot_fullscreen = true);
    }

    /// The window shows on every ordinary desktop of its screen.
    pub fn everywhere(&self, id: u32) {
        self.edit(id, |window| window.everywhere = true);
    }

    pub fn retitle(&self, id: u32, title: &str) {
        self.edit(id, |window| window.title = title.into());
    }

    // ---- what a person does to the desktop (not logged)

    fn screen_of_space(&self, space: u64) -> usize {
        self.screens
            .borrow()
            .iter()
            .position(|screen| screen.row.iter().any(|s| s.id == space))
            .expect("every window is on some screen")
    }

    fn enter_fullscreen(&self, id: u32) {
        let space = self.new_space();
        let mut windows = self.windows.borrow_mut();
        let window = windows.iter_mut().find(|w| w.id == id).expect("window");
        let index = self.screen_of_space(window.space);
        let mut screens = self.screens.borrow_mut();
        let screen = &mut screens[index];
        screen.row.push(Space {
            id: space,
            fullscreen: true,
        });
        if self.switch_lag.get() == 0 {
            screen.current = space;
        } else {
            self.pending
                .borrow_mut()
                .push((self.switch_lag.get(), Later::Show(index, space)));
        }
        window.windowed_frame = window.frame;
        window.frame = screen.bounds;
        window.came_from = window.space;
        window.space = space;
        window.fullscreen = true;
        window.listed = !window.withheld;
    }

    fn leave_fullscreen(&self, id: u32) {
        let mut windows = self.windows.borrow_mut();
        let window = windows.iter_mut().find(|w| w.id == id).expect("window");
        let index = self.screen_of_space(window.space);
        let mut screens = self.screens.borrow_mut();
        let screen = &mut screens[index];
        let in_view = screen.current == window.space;
        screen.row.retain(|space| space.id != window.space);
        let still_there = screen.row.iter().any(|space| space.id == window.came_from);
        if in_view || !still_there {
            screen.current = screen.last_ordinary;
            window.space = screen.last_ordinary;
        } else {
            window.space = window.came_from;
        }
        window.frame = window.windowed_frame;
        window.fullscreen = false;
    }

    fn toggle_fullscreen(&self, id: u32, on: bool) {
        let is_on = self
            .windows
            .borrow()
            .iter()
            .any(|window| window.id == id && window.fullscreen);
        match (is_on, on) {
            (false, true) => self.enter_fullscreen(id),
            (true, false) => self.leave_fullscreen(id),
            _ => {}
        }
    }

    /// The person makes the window full screen (or takes it out).
    pub fn user_fullscreen(&self, id: u32, on: bool) {
        self.toggle_fullscreen(id, on);
    }

    /// The person drags the window to ordinary desktop `desktop` of `screen`, at `frame`.
    pub fn user_moves(&self, id: u32, screen: &str, desktop: usize, frame: Rect) {
        let space = self.ordinary_space(screen, desktop);
        self.edit(id, |window| {
            window.space = space;
            window.frame = frame;
            window.windowed_frame = frame;
        });
    }

    /// The person switches `screen` to the desktop at `position` in its row (from 1;
    /// full-screen desktops included).
    pub fn user_shows(&self, screen: &str, position: usize) {
        let index = self.screen_index(screen);
        let mut screens = self.screens.borrow_mut();
        let space = screens[index].row[position - 1].clone();
        screens[index].current = space.id;
        if !space.fullscreen {
            screens[index].last_ordinary = space.id;
        }
    }

    /// The person closes the last ordinary desktop of `screen` (which must be empty and
    /// not in view).
    pub fn user_closes_last_desktop(&self, screen: &str) {
        let index = self.screen_index(screen);
        let mut screens = self.screens.borrow_mut();
        let last = screens[index]
            .row
            .iter()
            .rposition(|space| !space.fullscreen)
            .expect("an ordinary desktop");
        screens[index].row.remove(last);
    }

    // ---- what the tests look at

    fn place_text(&self, space: u64) -> String {
        let screens = self.screens.borrow();
        let screen = &screens[self.screen_of_space(space)];
        let mut number = 0;
        for candidate in &screen.row {
            if !candidate.fullscreen {
                number += 1;
            }
            if candidate.id == space {
                return if candidate.fullscreen {
                    format!("{} full screen", screen.name)
                } else {
                    format!("{} desktop {number}", screen.name)
                };
            }
        }
        unreachable!()
    }

    fn find<T>(&self, id: u32, read: impl FnOnce(&Window) -> T) -> T {
        let windows = self.windows.borrow();
        let window = windows
            .iter()
            .find(|window| window.id == id)
            .unwrap_or_else(|| panic!("no window {id}"));
        read(window)
    }

    /// `Main desktop 2` or `Side full screen`.
    pub fn where_is(&self, id: u32) -> String {
        self.place_text(self.find(id, |window| window.space))
    }

    pub fn frame_of(&self, id: u32) -> Rect {
        self.find(id, |window| window.frame)
    }

    /// The row of `screen`, the desktop in view in brackets: `1 [2] F:Safari`.
    pub fn row(&self, screen: &str) -> String {
        let screens = self.screens.borrow();
        let screen = &screens[self.screen_index(screen)];
        let mut number = 0;
        let cells: Vec<String> = screen
            .row
            .iter()
            .map(|space| {
                let cell = if space.fullscreen {
                    let owner = self
                        .windows
                        .borrow()
                        .iter()
                        .find(|window| window.space == space.id)
                        .map(|window| format!("{} {}", window.app, window.title))
                        .unwrap_or_default();
                    format!("F:{owner}")
                } else {
                    number += 1;
                    number.to_string()
                };
                if space.id == screen.current {
                    format!("[{cell}]")
                } else {
                    cell
                }
            })
            .collect();
        cells.join(" ")
    }

    /// Every window as `app|title|place|x,y w×h`, sorted — to compare two states.
    pub fn snapshot(&self) -> Vec<String> {
        let mut lines: Vec<String> = self
            .windows
            .borrow()
            .iter()
            .map(|window| {
                format!(
                    "{}|{}|{}|{:.0},{:.0} {:.0}×{:.0}",
                    window.app,
                    window.title,
                    self.place_text(window.space),
                    window.frame.x,
                    window.frame.y,
                    window.frame.width,
                    window.frame.height
                )
            })
            .collect();
        lines.sort();
        lines
    }

    /// What was done to the desktop, leaving out the by-number lookups.
    pub fn actions(&self) -> Vec<String> {
        self.log
            .borrow()
            .iter()
            .filter(|entry| !entry.starts_with("scan"))
            .cloned()
            .collect()
    }

    /// How many log entries start with `what` (`step`, `fullscreen`, `move`, `park`, `scan`).
    pub fn count(&self, what: &str) -> usize {
        self.log
            .borrow()
            .iter()
            .filter(|entry| entry.starts_with(what))
            .count()
    }

    /// Forgets what was done so far, so that a test sees only what comes next.
    pub fn clear_log(&self) {
        self.log.borrow_mut().clear();
        self.waited_ms.set(0);
        self.looks.set(0);
    }

    /// Lets time pass, so that whatever macOS is still in the middle of gets finished.
    pub fn time_passes(&self) {
        let waited = self.waited_ms.get();
        (0..50).for_each(|_| self.pause(0));
        self.waited_ms.set(waited);
    }

    /// Puts the pointer somewhere without it counting as something done.
    pub fn pointer_at(&self, x: f64, y: f64) {
        self.pointer.set((x, y));
    }

    // ---- helpers for the port

    fn in_view(&self, window: &Window) -> bool {
        let screens = self.screens.borrow();
        let screen = &screens[self.screen_of_space(window.space)];
        screen.current == window.space
            || (window.everywhere && screen.current == screen.last_ordinary)
    }

    fn window_id(&self, handle: usize) -> Option<u32> {
        self.handles.borrow().get(handle).copied()
    }

    fn spaces_of(&self, window: &Window) -> Vec<u64> {
        if !window.everywhere {
            return vec![window.space];
        }
        let screens = self.screens.borrow();
        screens[self.screen_of_space(window.space)]
            .row
            .iter()
            .filter(|space| !space.fullscreen)
            .map(|space| space.id)
            .collect()
    }

    fn describe(&self, window: &Window, handle: usize) -> RawWindow {
        RawWindow {
            app: Some(window.app.clone()),
            bundle_id: Some(format!("com.sim.{}", window.app.to_lowercase())),
            pid: window.pid,
            handle,
            window_id: Some(window.id),
            subrole: Some(STANDARD_WINDOW.into()),
            minimized: Some(false),
            fullscreen: Some(window.fullscreen),
            title: Some(window.title.clone()),
            position: Some((window.frame.x, window.frame.y)),
            size: Some((window.frame.width, window.frame.height)),
            spaces: self.spaces_of(window),
        }
    }

    /// Hands the windows with these ids out, giving each the next handle.
    fn hand_out(&self, ids: Vec<u32>) -> Vec<RawWindow> {
        ids.into_iter()
            .map(|id| {
                self.edit(id, |window| window.listed = true);
                let handle = {
                    let mut handles = self.handles.borrow_mut();
                    handles.push(id);
                    handles.len() - 1
                };
                self.find(id, |window| self.describe(window, handle))
            })
            .collect()
    }

    fn note(&self, entry: String) {
        self.log.borrow_mut().push(entry);
    }

    /// The screen holding this point, if any.
    fn screen_at(&self, x: f64, y: f64) -> Option<usize> {
        self.screens.borrow().iter().position(|screen| {
            x >= screen.bounds.x
                && x < screen.bounds.max_x()
                && y >= screen.bounds.y
                && y < screen.bounds.max_y()
        })
    }
}

impl WindowSystem for Sim {
    fn is_trusted(&self, _prompt: bool) -> bool {
        self.trusted.get()
    }

    fn displays(&self) -> Vec<RawDisplay> {
        self.screens
            .borrow()
            .iter()
            .map(|screen| RawDisplay {
                name: Some(screen.name.clone()),
                vendor: 1,
                model: 2,
                serial: screen.serial,
                uuid: Some(format!("uuid-{}", screen.serial)),
                bounds: screen.bounds,
            })
            .collect()
    }

    fn windows(&self) -> Vec<RawWindow> {
        self.looks.set(self.looks.get() + 1);
        self.handles.borrow_mut().clear();
        let visible: Vec<u32> = self
            .windows
            .borrow()
            .iter()
            .filter(|window| self.in_view(window) && !window.withheld)
            .map(|window| window.id)
            .collect();
        self.hand_out(visible)
    }

    fn numbered_windows(&self, wanted: &[(i32, u32)]) -> Vec<RawWindow> {
        self.note(format!("scan {}", wanted.len()));
        let found: Vec<u32> = self
            .windows
            .borrow()
            .iter()
            .filter(|window| wanted.contains(&(window.pid, window.id)) && window.listed)
            .map(|window| window.id)
            .collect();
        self.hand_out(found)
    }

    fn server_windows(&self) -> Vec<ServerWindow> {
        self.windows
            .borrow()
            .iter()
            .map(|window| ServerWindow {
                app: Some(window.app.clone()),
                bundle_id: Some(format!("com.sim.{}", window.app.to_lowercase())),
                pid: window.pid,
                window_id: window.id,
                layer: 0,
                alpha: 1.0,
                bounds: window.frame,
                spaces: self.spaces_of(window),
            })
            .collect()
    }

    fn screens(&self) -> Vec<RawScreen> {
        self.screens
            .borrow()
            .iter()
            .map(|screen| RawScreen {
                uuid: format!("uuid-{}", screen.serial),
                current: screen.current,
                spaces: screen
                    .row
                    .iter()
                    .map(|space| RawSpace {
                        id: space.id,
                        fullscreen: space.fullscreen,
                    })
                    .collect(),
            })
            .collect()
    }

    fn set_frame(&self, handle: usize, frame: Rect) -> bool {
        let Some(id) = self.window_id(handle) else {
            return false;
        };
        let target_screen = self.screen_at(frame.x, frame.y);
        let mut windows = self.windows.borrow_mut();
        let window = windows.iter_mut().find(|w| w.id == id).expect("window");
        self.log
            .borrow_mut()
            .push(format!("move {} {}", window.app, window.title));
        if window.fullscreen || window.refuses_to_move {
            return false;
        }
        let width = if window.stubborn > 0 {
            window.stubborn -= 1;
            window.frame.width
        } else {
            frame.width
        };
        window.frame = Rect::new(frame.x, frame.y, width, frame.height);
        window.windowed_frame = window.frame;
        let from = self.screen_of_space(window.space);
        if let Some(to) = target_screen.filter(|to| *to != from) {
            let screens = self.screens.borrow();
            let screen = &screens[to];
            let showing_fullscreen = screen
                .row
                .iter()
                .any(|space| space.id == screen.current && space.fullscreen);
            window.space = if showing_fullscreen {
                screen.last_ordinary
            } else {
                screen.current
            };
        }
        true
    }

    fn frame(&self, handle: usize) -> Option<Rect> {
        self.window_id(handle).map(|id| self.frame_of(id))
    }

    fn set_fullscreen(&self, handle: usize, on: bool) -> bool {
        let Some(id) = self.window_id(handle) else {
            return false;
        };
        self.note(format!("fullscreen {} {id}", if on { "on" } else { "off" }));
        if self.find(id, |window| window.cannot_fullscreen) {
            return false;
        }
        self.front.set(0);
        if self.fullscreen_lag.get() == 0 {
            self.toggle_fullscreen(id, on);
        } else {
            self.pending
                .borrow_mut()
                .push((self.fullscreen_lag.get(), Later::Toggle(id, on)));
        }
        true
    }

    fn desktop_shortcuts(&self) -> [Shortcut; 2] {
        let setting = Shortcut {
            enabled: Some(self.shortcuts_on.get()),
            key: None,
            modifiers: None,
        };
        [setting; 2]
    }

    fn press_desktop_key(&self, right: bool) {
        let (x, y) = self.pointer.get();
        let Some(index) = self.screen_at(x, y) else {
            return;
        };
        let mut screens = self.screens.borrow_mut();
        let screen = &mut screens[index];
        self.log.borrow_mut().push(format!(
            "step {} {}",
            screen.name,
            if right { "right" } else { "left" }
        ));
        if !self.keys_work.get() {
            return;
        }
        let position = screen
            .row
            .iter()
            .position(|space| space.id == screen.current)
            .expect("the desktop in view is in the row");
        let next = if right {
            position + 1
        } else {
            position.wrapping_sub(1)
        };
        if let Some(space) = screen.row.get(next).cloned() {
            self.front.set(0);
            screen.current = space.id;
            if !space.fullscreen {
                screen.last_ordinary = space.id;
            }
        }
    }

    fn pointer(&self) -> (f64, f64) {
        self.pointer.get()
    }

    fn park_pointer(&self, x: f64, y: f64) {
        self.note(format!("park {x:.0},{y:.0}"));
        self.pointer.set((x, y));
    }

    fn pause(&self, milliseconds: u64) {
        self.waited_ms.set(self.waited_ms.get() + milliseconds);
        let due: Vec<Later> = {
            let mut pending = self.pending.borrow_mut();
            pending.iter_mut().for_each(|entry| entry.0 -= 1);
            let due = pending
                .iter()
                .filter(|entry| entry.0 == 0)
                .map(|entry| entry.1)
                .collect();
            pending.retain(|entry| entry.0 > 0);
            due
        };
        for event in due {
            match event {
                Later::Toggle(id, on) => self.toggle_fullscreen(id, on),
                Later::Show(screen, space) => self.screens.borrow_mut()[screen].current = space,
            }
        }
    }

    fn front_app(&self) -> Option<i32> {
        Some(self.front.get())
    }

    fn activate(&self, pid: i32) {
        self.note(format!("activate {pid}"));
        self.front.set(pid);
    }
}
