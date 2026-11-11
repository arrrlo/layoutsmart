//! Puts windows back the way a profile has them: on their screen and desktop, at their
//! frame, in or out of full screen.
//!
//! It works by looking and correcting. Each round lists every window, pairs the windows
//! with the profile, and does one kind of thing to those that need it — first reaching
//! windows that cannot be reached yet, then taking windows out of full screen, then
//! placing, then making windows full screen. Then it looks again, because each of those
//! changes where things are. A window is tried once for each kind of thing; one that is
//! still not right afterwards is reported, not tried again.
//!
//! What macOS allows from outside an app decides how each thing is done:
//!
//! * a window lands on the desktop its screen is showing when it is moved onto that
//!   screen from another one, or when it leaves full screen — nothing else moves a
//!   window between desktops;
//! * a screen shows another desktop only when the "next desktop" keyboard shortcut is
//!   pressed while the pointer is on it;
//! * an app hands out a window only once its desktop has been in view.

use crate::desktops::{DesktopMap, RawScreen, steps_with_control_arrows};
use crate::displays::{RawDisplay, display_identity};
use crate::geometry::Rect;
use crate::matcher::{Open, Pair, Step, pair};
use crate::model::{Layout, SavedScreen, Showing, WindowInfo};
use crate::ports::WindowSystem;
use crate::service::{self, Backend, Error};
use std::cmp::Ordering;

/// Said when a desktop had to be brought into view and could not be.
pub const NO_STEPPING_NOTE: &str = "Layoutsmart could not switch desktops, so some windows \
     may be on the wrong desktop. It switches with the keyboard shortcuts “Move left a \
     space” and “Move right a space” (Control–← and Control–→): check that they are on \
     and unchanged in System Settings → Keyboard → Keyboard Shortcuts → Mission Control.";

/// More rounds than any window needs (reach, leave, place, enter, and a last look).
const MAX_ROUNDS: usize = 8;
/// How often a window is asked to take its frame before that is taken as its answer.
const PASSES: usize = 3;
/// How long, and how often, to wait for macOS to finish an animation.
pub(crate) const POLL_MS: u64 = 100;
pub(crate) const POLLS: usize = 30;
/// Extra time after a screen has arrived at a desktop, before windows are moved onto it.
pub(crate) const STEP_SETTLE_MS: u64 = 300;
/// Extra time after a window has entered or left full screen.
pub(crate) const FULLSCREEN_SETTLE_MS: u64 = 700;

/// The order in which the kinds of step are done.
const ORDER: [Step; 4] = [Step::Reach, Step::Leave, Step::Place, Step::Enter];

/// A handle that names no window. A window without a handle is only ever reached for,
/// which uses no handle.
const NO_HANDLE: usize = usize::MAX;

/// One window, for telling the user about it.
#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    pub app: String,
    pub title: String,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Report {
    /// Windows that were put back.
    pub moved: Vec<Entry>,
    /// Windows that are not where the profile has them, although it was tried.
    pub failed: Vec<Entry>,
    /// Things the user should know about what could not be done.
    pub notes: Vec<String>,
}

/// The desktop as one look finds it.
struct Scene {
    open: Vec<Open>,
    desktops: DesktopMap,
    /// False when either the profile or macOS knows nothing about desktops: then only
    /// ordinary windows in view are handled, as in version 0.1.
    whole: bool,
}

/// Entry of the profile and window number: one saved window taken to be one open window.
type Key = (usize, Option<u32>);

/// One thing to do to one window.
struct Job {
    key: Key,
    handle: usize,
    step: Step,
    entry: Entry,
    /// Where the profile has it.
    frame: Rect,
    /// Where it is.
    now: Rect,
    /// The desktop it is on: (display, position in the row).
    place: Option<(String, usize)>,
    /// The screen it belongs on, if that is connected.
    screen: Option<String>,
    on_screen: bool,
    /// The desktop to bring into view before moving it, when it is on the wrong one.
    home: Option<(String, usize)>,
}

impl Job {
    fn new(pair: &Pair, map: &DesktopMap) -> Option<Job> {
        let step = pair.step(map)?;
        let open = pair.open;
        Some(Job {
            key: (pair.order, open.id),
            handle: open.handle.unwrap_or(NO_HANDLE),
            step,
            entry: Entry {
                app: open.app.clone(),
                title: open.title.clone(),
            },
            frame: pair.saved.frame(),
            now: open.frame,
            place: open
                .place
                .as_ref()
                .map(|place| (place.display.clone(), place.row)),
            screen: pair.screen(map).map(|row| row.display.clone()),
            on_screen: pair.on_screen(map),
            home: pair.home(map),
        })
    }

    /// True unless all that is wrong is the frame.
    fn misplaced(&self) -> bool {
        self.step != Step::Place || !self.on_screen || self.home.is_some()
    }

    fn mark(&self) -> Mark {
        Mark {
            key: self.key,
            step: self.step,
            entry: self.entry.clone(),
        }
    }
}

/// A record that something was done to a window.
struct Mark {
    key: Key,
    step: Step,
    entry: Entry,
}

struct Engine<'a> {
    sys: &'a dyn WindowSystem,
    layout: &'a Layout,
    displays: Vec<RawDisplay>,
    /// False once it is known that desktops cannot be switched.
    can_step: bool,
    /// Where the pointer was before it was first moved.
    came_from: Option<(f64, f64)>,
    front: Option<i32>,
    /// True once something was done that takes the user's app out of front.
    disturbed: bool,
    marks: Vec<Mark>,
    failed: Vec<Key>,
    notes: Vec<String>,
}

fn identity(display: &RawDisplay) -> String {
    display_identity(display.vendor, display.model, display.serial)
}

fn fullscreen_desktops(screens: &[RawScreen]) -> usize {
    screens
        .iter()
        .flat_map(|screen| &screen.spaces)
        .filter(|space| space.fullscreen)
        .count()
}

/// Whether macOS has finished taking a window into (`on`) or out of full screen, going
/// by the desktops before and now. Going in, it makes the new desktop at once but shows
/// it only when its animation is over; coming out, the desktop goes when it is over.
fn transition_is_over(before: &[RawScreen], now: &[RawScreen], on: bool) -> bool {
    if on {
        let existed = |id: u64| {
            before
                .iter()
                .flat_map(|screen| &screen.spaces)
                .any(|space| space.id == id)
        };
        now.iter().any(|screen| !existed(screen.current))
    } else {
        fullscreen_desktops(now) < fullscreen_desktops(before)
    }
}

/// Where in its row the full-screen window of this app on this display is: the one with
/// this title, or else the first.
fn fullscreen_row(scene: &Scene, display: &str, bundle_id: &str, title: &str) -> Option<usize> {
    let there: Vec<&Open> = scene
        .open
        .iter()
        .filter(|window| {
            window.fullscreen && window.bundle_id == bundle_id && window.is_on(display)
        })
        .collect();
    there
        .iter()
        .find(|window| window.title == title)
        .or(there.first())
        .and_then(|window| window.place.as_ref())
        .map(|place| place.row)
}

/// The desktop a screen is to show in the end — what it showed when the profile was
/// saved — as (display, position in the row).
fn view(saved: &SavedScreen, scene: &Scene) -> Option<(String, usize)> {
    let row = scene.desktops.row(&saved.display)?;
    let index = match &saved.showing {
        Showing::Desktop(number) => row.row_of((*number).min(row.desktops())),
        Showing::FullScreen { bundle_id, title } => {
            fullscreen_row(scene, &saved.display, bundle_id, title)
        }
        Showing::Unknown => None,
    };
    index.map(|index| (saved.display.clone(), index))
}

impl<'a> Engine<'a> {
    fn new(sys: &'a dyn WindowSystem, layout: &'a Layout) -> Engine<'a> {
        Engine {
            sys,
            layout,
            displays: sys.displays(),
            can_step: steps_with_control_arrows(&sys.desktop_shortcuts()),
            came_from: None,
            front: sys.front_app(),
            disturbed: false,
            marks: Vec::new(),
            failed: Vec::new(),
            notes: Vec::new(),
        }
    }

    fn look(&self, backend: &Backend) -> Result<Scene, Error> {
        let inventory = service::inventory(backend)?;
        let whole = self.layout.knows_desktops() && inventory.desktops.is_known();
        Ok(Scene {
            open: inventory.open(whole),
            desktops: inventory.desktops,
            whole,
        })
    }

    // ---- small things the steps are made of

    /// Waits until `done`, for as long as an animation may take. False if it never was.
    fn wait(&self, done: impl Fn(&Engine) -> bool) -> bool {
        (0..POLLS).any(|_| {
            self.sys.pause(POLL_MS);
            done(self)
        })
    }

    /// Position, in its row, of the desktop this display is showing.
    fn showing(&self, display: &str) -> Option<usize> {
        DesktopMap::new(&self.displays, &self.sys.screens())
            .row(display)
            .and_then(|row| row.showing)
    }

    fn bounds(&self, display: &str) -> Option<Rect> {
        self.displays
            .iter()
            .find(|candidate| identity(candidate) == display)
            .map(|display| display.bounds)
    }

    /// Puts the pointer in the middle of a display, which is how macOS is told which
    /// screen a shortcut is meant for.
    fn park(&mut self, display: &str) -> bool {
        let bounds = self.bounds(display);
        bounds.iter().for_each(|bounds| {
            self.came_from.get_or_insert(self.sys.pointer());
            self.sys.park_pointer(
                bounds.x + bounds.width / 2.0,
                bounds.y + bounds.height / 2.0,
            );
        });
        bounds.is_some()
    }

    fn press(&mut self, display: &str, right: bool) -> bool {
        let from = self.showing(display);
        self.disturbed = true;
        self.sys.press_desktop_key(right);
        self.wait(|engine| engine.showing(display) != from)
    }

    /// Moves a screen one desktop along. The first time that does not work is the last
    /// time it is tried.
    fn step(&mut self, display: &str, right: bool) -> bool {
        let moved = self.can_step && self.park(display) && self.press(display, right);
        if !moved {
            self.can_step = false;
            if self.notes.is_empty() {
                self.notes.push(NO_STEPPING_NOTE.to_string());
            }
        }
        moved
    }

    fn walk(&mut self, display: &str, from: usize, to: usize) -> bool {
        let right = to.cmp(&from) == Ordering::Greater;
        let arrived = (0..to.abs_diff(from)).all(|_| self.step(display, right));
        if arrived && to != from {
            self.sys.pause(STEP_SETTLE_MS);
        }
        arrived
    }

    /// Brings the desktop at position `row` of a display into view. False if it could
    /// not be.
    fn show(&mut self, display: &str, row: usize) -> bool {
        self.showing(display)
            .is_some_and(|now| self.walk(display, now, row))
    }

    /// Switches a window into or out of full screen and waits for macOS to finish.
    /// True if it happened.
    fn toggle(&mut self, handle: usize, on: bool) -> bool {
        let before = self.sys.screens();
        if !self.sys.set_fullscreen(handle, on) {
            return false;
        }
        self.disturbed = true;
        let changed = self.wait(|engine| transition_is_over(&before, &engine.sys.screens(), on));
        self.sys.pause(FULLSCREEN_SETTLE_MS);
        changed
    }

    /// Moves and resizes a window until it is where it should be — some apps need
    /// asking more than once. True if the app took the request.
    fn settle(&self, handle: usize, frame: Rect) -> bool {
        let mut accepted = false;
        for _ in 0..PASSES {
            accepted = self.sys.set_frame(handle, frame);
            let there = self
                .sys
                .frame(handle)
                .is_some_and(|now| now.integral_eq(&frame));
            if there || !accepted {
                break;
            }
        }
        accepted
    }

    // ---- the steps

    /// Shows each desktop that holds a window no app has handed out, and asks the apps
    /// for their windows while it is in view. After that they can be found.
    fn reach(&mut self, jobs: &[Job]) {
        let mut desktops: Vec<&(String, usize)> =
            jobs.iter().filter_map(|job| job.place.as_ref()).collect();
        desktops.sort();
        desktops.dedup();
        for (display, row) in desktops {
            if self.show(display, *row) {
                self.sys.windows();
            }
        }
    }

    /// A place for the window on some other display than the one it belongs on.
    fn elsewhere(&self, job: &Job) -> Option<Rect> {
        self.displays
            .iter()
            .find(|display| job.screen.as_deref() != Some(identity(display).as_str()))
            .map(|display| job.frame.centred_in(&display.bounds))
    }

    /// Takes a window off its screen's desktops, so that coming back puts it on the one
    /// in view: by way of another screen, or — with a single screen — of full screen.
    fn send_away(&mut self, job: &Job) {
        match self.elsewhere(job) {
            Some(away) => {
                self.sys.set_frame(job.handle, away);
            }
            None => {
                let _ = self.toggle(job.handle, true) && self.toggle(job.handle, false);
            }
        }
    }

    fn place_one(&mut self, job: &Job) {
        let ready = job
            .home
            .as_ref()
            .is_some_and(|(display, row)| self.show(display, *row));
        if ready && job.on_screen {
            self.send_away(job);
        }
        if !self.settle(job.handle, job.frame) {
            self.failed.push(job.key);
        }
    }

    /// Puts windows on their desktops, desktop by desktop, at their frames. True if any
    /// of them had to change desktop or screen, which a further look has to confirm.
    fn place(&mut self, jobs: &[Job]) -> bool {
        let mut sorted: Vec<&Job> = jobs.iter().collect();
        sorted.sort_by(|a, b| a.home.cmp(&b.home));
        sorted.into_iter().for_each(|job| self.place_one(job));
        jobs.iter().any(Job::misplaced)
    }

    /// Makes a window full screen, on the screen it belongs on.
    fn enter(&mut self, job: &Job) {
        if !job.on_screen {
            let screen = job
                .screen
                .as_deref()
                .and_then(|display| self.bounds(display));
            screen.iter().for_each(|bounds| {
                self.sys.set_frame(job.handle, job.now.centred_in(bounds));
            });
        }
        self.toggle(job.handle, true);
    }

    /// True if another look is needed to know how things stand.
    fn perform(&mut self, step: Step, jobs: &[Job]) -> bool {
        match step {
            Step::Reach => self.reach(jobs),
            Step::Leave => jobs.iter().for_each(|job| {
                self.toggle(job.handle, false);
            }),
            Step::Place => return self.place(jobs),
            Step::Enter => jobs.iter().for_each(|job| self.enter(job)),
        }
        true
    }

    // ---- one round

    fn tried(&self, job: &Job) -> bool {
        self.marks
            .iter()
            .any(|mark| mark.key == job.key && mark.step == job.step)
    }

    /// Leaves out what was tried before; of that, what is still in the wrong place has
    /// failed. (A frame that is still off after its app took the request is as close as
    /// that app allows.)
    fn fresh(&mut self, jobs: Vec<Job>) -> Vec<Job> {
        let (stale, fresh): (Vec<Job>, Vec<Job>) =
            jobs.into_iter().partition(|job| self.tried(job));
        self.failed.extend(
            stale
                .iter()
                .filter(|job| job.misplaced())
                .map(|job| job.key),
        );
        fresh
    }

    /// Does the most urgent kind of thing windows need. True if it is worth looking again.
    fn act(&mut self, scene: &Scene) -> bool {
        let wanted: Vec<&WindowInfo> = self
            .layout
            .windows
            .iter()
            .filter(|window| scene.whole || !window.fullscreen)
            .collect();
        let jobs: Vec<Job> = pair(&wanted, &scene.open)
            .iter()
            .filter_map(|pair| Job::new(pair, &scene.desktops))
            .collect();
        let jobs = self.fresh(jobs);
        let Some(step) = ORDER
            .into_iter()
            .find(|step| jobs.iter().any(|job| job.step == *step))
        else {
            return false;
        };
        let (now, later): (Vec<Job>, Vec<Job>) = jobs.into_iter().partition(|job| job.step == step);
        self.marks.extend(now.iter().map(Job::mark));
        self.perform(step, &now) || !later.is_empty()
    }

    // ---- afterwards

    /// Leaves each screen showing what it showed when the profile was saved, the pointer
    /// where it was and the user's app in front (if it can be seen there).
    fn finish(&mut self, scene: &Scene) {
        let layout = self.layout;
        layout
            .desktops
            .iter()
            .filter_map(|screen| view(screen, scene))
            .for_each(|(display, row)| {
                self.show(&display, row);
            });
        self.came_from
            .iter()
            .for_each(|(x, y)| self.sys.park_pointer(*x, *y));
        if self.disturbed {
            let front = self.front.filter(|pid| self.has_window_in_view(*pid));
            front.iter().for_each(|pid| self.sys.activate(*pid));
        }
    }

    /// Whether an app has a window on a desktop that is in view. Bringing an app forward
    /// that has none makes macOS switch to a desktop where it has one — which would undo
    /// what the screens were just set to show.
    fn has_window_in_view(&self, pid: i32) -> bool {
        let in_view: Vec<u64> = self
            .sys
            .screens()
            .iter()
            .map(|screen| screen.current)
            .collect();
        self.sys.server_windows().iter().any(|window| {
            window.pid == pid
                && window.layer == 0
                && window.spaces.iter().any(|id| in_view.contains(id))
        })
    }

    fn report(self) -> Report {
        // The last thing done to each window, in the order of the profile.
        let mut latest: Vec<&Mark> = Vec::new();
        for mark in self.marks.iter().rev() {
            if !latest.iter().any(|seen| seen.key == mark.key) {
                latest.push(mark);
            }
        }
        latest.sort_by_key(|mark| mark.key.0);
        let failed = |mark: &&&Mark| self.failed.contains(&mark.key);
        Report {
            moved: latest
                .iter()
                .filter(|mark| mark.step != Step::Reach && !failed(mark))
                .map(|mark| mark.entry.clone())
                .collect(),
            failed: latest
                .iter()
                .filter(failed)
                .map(|mark| mark.entry.clone())
                .collect(),
            notes: self.notes.clone(),
        }
    }
}

/// Restores `layout`. Fails only if windows cannot be read at all.
pub fn restore(backend: &Backend, layout: &Layout) -> Result<Report, Error> {
    let mut engine = Engine::new(backend.sys, layout);
    let mut scene = None;
    for _ in 0..MAX_ROUNDS {
        let seen = scene.insert(engine.look(backend)?);
        if !engine.act(seen) {
            break;
        }
    }
    scene.iter().for_each(|scene| engine.finish(scene));
    Ok(engine.report())
}

#[cfg(test)]
pub(crate) mod testing;
#[cfg(test)]
mod tests;
