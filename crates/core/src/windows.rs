//! Decides which of the windows macOS reports are ones Layoutsmart manages.

use crate::desktops::{DesktopMap, Place};
use crate::geometry::Rect;

/// Accessibility subrole of an ordinary document/app window.
pub const STANDARD_WINDOW: &str = "AXStandardWindow";

/// Smallest window-server window taken for a real window (helper windows are tiny).
pub const MIN_WIDTH: f64 = 100.0;
pub const MIN_HEIGHT: f64 = 50.0;

/// One window exactly as the Accessibility API reports it. Any field may be missing.
#[derive(Debug, Clone, PartialEq)]
pub struct RawWindow {
    pub app: Option<String>,
    pub bundle_id: Option<String>,
    pub pid: i32,
    /// What the macOS layer calls this window; passed back to move it.
    pub handle: usize,
    /// The window server's number for this window, if known.
    pub window_id: Option<u32>,
    pub subrole: Option<String>,
    pub minimized: Option<bool>,
    pub fullscreen: Option<bool>,
    pub title: Option<String>,
    pub position: Option<(f64, f64)>,
    pub size: Option<(f64, f64)>,
    /// Ids of the desktops the window is on (empty when unknown).
    pub spaces: Vec<u64>,
}

/// A window as the window server lists it. The server knows every window on every
/// desktop, but only where it is — not its title or kind, and it cannot move it.
#[derive(Debug, Clone, PartialEq)]
pub struct ServerWindow {
    pub app: Option<String>,
    pub bundle_id: Option<String>,
    pub pid: i32,
    pub window_id: u32,
    /// 0 for ordinary windows; menus, the Dock and overlays live on other layers.
    pub layer: i32,
    pub alpha: f64,
    pub bounds: Rect,
    pub spaces: Vec<u64>,
}

/// A window Layoutsmart can save and move.
#[derive(Debug, Clone, PartialEq)]
pub struct LiveWindow {
    pub handle: usize,
    pub window_id: Option<u32>,
    pub app: String,
    pub bundle_id: String,
    pub title: String,
    pub frame: Rect,
    pub fullscreen: bool,
    /// Screen and desktop, when macOS tells.
    pub place: Option<Place>,
}

impl LiveWindow {
    /// An ordinary window on a desktop that is in view: all that version 0.1 handled.
    pub fn is_plain_in_view(&self) -> bool {
        !self.fullscreen && self.place.as_ref().is_none_or(|place| place.showing)
    }
}

/// A window that exists on some desktop but that Accessibility cannot reach: macOS only
/// hands a window out once its desktop has been in view while someone asked for it.
#[derive(Debug, Clone, PartialEq)]
pub struct Unreached {
    pub window_id: u32,
    pub pid: i32,
    pub app: String,
    pub bundle_id: String,
    pub frame: Rect,
    pub place: Place,
}

/// (app name, bundle id), falling back on the process id and then on the app name.
fn names(app: &Option<String>, bundle_id: &Option<String>, pid: i32) -> (String, String) {
    let app = app.clone().unwrap_or_else(|| format!("pid {pid}"));
    let bundle_id = bundle_id.clone().unwrap_or_else(|| app.clone());
    (app, bundle_id)
}

impl RawWindow {
    fn is_managed(&self) -> bool {
        self.subrole.as_deref() == Some(STANDARD_WINDOW) && self.minimized != Some(true)
    }

    fn frame(&self) -> Option<Rect> {
        let (x, y) = self.position?;
        let (width, height) = self.size?;
        Some(Rect::new(x, y, width, height))
    }

    fn to_live(&self, map: &DesktopMap) -> Option<LiveWindow> {
        let frame = self.frame()?;
        let (app, bundle_id) = names(&self.app, &self.bundle_id, self.pid);
        let place = map.place(&self.spaces);
        // Where its desktop is known, that decides: an app says "full screen" as soon as
        // it is asked to be, before macOS has made the full-screen desktop, and goes on
        // saying it for a while if macOS never does.
        let fullscreen = place
            .as_ref()
            .map_or(self.fullscreen == Some(true), |place| {
                place.kind.is_fullscreen()
            });
        Some(LiveWindow {
            handle: self.handle,
            window_id: self.window_id,
            app,
            bundle_id,
            title: self.title.clone().unwrap_or_default(),
            frame,
            fullscreen,
            place,
        })
    }
}

/// Standard, non-minimised windows with a known frame, on any desktop.
///
/// When macOS tells which desktops exist, a window on none of them is left out: it is a
/// background tab (every tab is a window of its own; only the selected one is on a desktop).
pub fn live_windows(raw: &[RawWindow], map: &DesktopMap) -> Vec<LiveWindow> {
    raw.iter()
        .filter(|window| window.is_managed())
        .filter_map(|window| window.to_live(map))
        .filter(|window| window.place.is_some() || !map.is_known())
        .collect()
}

impl ServerWindow {
    fn looks_real(&self) -> bool {
        self.layer == 0
            && self.alpha > 0.0
            && self.bounds.width >= MIN_WIDTH
            && self.bounds.height >= MIN_HEIGHT
    }

    fn to_unreached(&self, map: &DesktopMap) -> Option<Unreached> {
        let place = map.place(&self.spaces)?;
        let (app, bundle_id) = names(&self.app, &self.bundle_id, self.pid);
        Some(Unreached {
            window_id: self.window_id,
            pid: self.pid,
            app,
            bundle_id,
            frame: self.bounds,
            place,
        })
    }
}

fn same_desktop(a: &Place, b: &Place) -> bool {
    a.display == b.display && a.row == b.row
}

fn area(window: &Unreached) -> f64 {
    window.frame.width * window.frame.height
}

/// A full-screen desktop holds one real window; macOS adds helper windows to it (a strip
/// for the title bar) that only the window server lists. So there, a server-only window
/// counts only if the desktop has no known window, and only the largest one.
fn is_real(candidate: &Unreached, all: &[Unreached], occupied: &[Place]) -> bool {
    if !candidate.place.kind.is_fullscreen() {
        return true;
    }
    let largest = all
        .iter()
        .filter(|other| same_desktop(&other.place, &candidate.place))
        .reduce(|best, other| {
            if area(other) > area(best) {
                other
            } else {
                best
            }
        });
    let taken = occupied
        .iter()
        .any(|place| same_desktop(place, &candidate.place));
    !taken && largest.is_some_and(|largest| largest.window_id == candidate.window_id)
}

/// Windows the window server lists on a desktop that Accessibility did not report.
pub fn unreached(raw: &[RawWindow], server: &[ServerWindow], map: &DesktopMap) -> Vec<Unreached> {
    let known: Vec<u32> = raw.iter().filter_map(|window| window.window_id).collect();
    let candidates: Vec<Unreached> = server
        .iter()
        .filter(|window| window.looks_real() && !known.contains(&window.window_id))
        .filter_map(|window| window.to_unreached(map))
        .collect();
    let occupied: Vec<Place> = live_windows(raw, map)
        .into_iter()
        .filter_map(|window| window.place)
        .collect();
    candidates
        .iter()
        .filter(|candidate| is_real(candidate, &candidates, &occupied))
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests;
