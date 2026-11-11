//! Pairs the windows a profile holds with the windows that are open now, and says what
//! each pair needs to be back the way it was saved.

use crate::desktops::{DesktopMap, Kind, Place, ScreenRow};
use crate::geometry::Rect;
use crate::model::WindowInfo;
use crate::windows::{LiveWindow, Unreached};

/// A window that is open now, on any desktop — as much of it as is known.
#[derive(Debug, Clone, PartialEq)]
pub struct Open {
    /// The window server's number for it: the same for as long as the window lives.
    pub id: Option<u32>,
    /// What the macOS layer calls it. `None`: its app has never handed it out, so it can
    /// be seen (where it is, how big) but not read or moved.
    pub handle: Option<usize>,
    pub app: String,
    pub bundle_id: String,
    /// Empty when the window cannot be reached.
    pub title: String,
    pub frame: Rect,
    pub fullscreen: bool,
    pub place: Option<Place>,
}

impl Open {
    pub fn reached(window: &LiveWindow) -> Open {
        Open {
            id: window.window_id,
            handle: Some(window.handle),
            app: window.app.clone(),
            bundle_id: window.bundle_id.clone(),
            title: window.title.clone(),
            frame: window.frame,
            fullscreen: window.fullscreen,
            place: window.place.clone(),
        }
    }

    pub fn unreached(window: &Unreached) -> Open {
        Open {
            id: Some(window.window_id),
            handle: None,
            app: window.app.clone(),
            bundle_id: window.bundle_id.clone(),
            title: String::new(),
            frame: window.frame,
            fullscreen: window.place.kind.is_fullscreen(),
            place: Some(window.place.clone()),
        }
    }

    pub fn is_on(&self, display: &str) -> bool {
        self.place
            .as_ref()
            .is_some_and(|place| place.display == display)
    }

    /// The window as a profile records it.
    pub fn to_saved(&self) -> WindowInfo {
        let info = WindowInfo::new(&self.app, &self.bundle_id, &self.title, self.frame);
        match &self.place {
            None => info,
            Some(place) if self.fullscreen => info.in_full_screen(&place.display),
            Some(Place {
                kind: Kind::Desktop(number),
                everywhere: false,
                display,
                ..
            }) => info.on_desktop(display, *number),
            Some(place) => info.on_display(&place.display),
        }
    }
}

/// 0 for ordinary windows; for full-screen ones, their desktop's position in its row.
fn rank(window: &Open) -> usize {
    window
        .place
        .as_ref()
        .filter(|_| window.fullscreen)
        .map_or(0, |place| place.row + 1)
}

/// Ordinary windows, then full-screen ones in the order of their desktops; within that,
/// oldest first (window numbers are handed out in the order windows are made).
///
/// macOS puts a new full-screen desktop at the end of its screen's row, so making
/// windows full screen in this order rebuilds the row. And the order macOS reports
/// windows in changes with the desktop in view and the window in front, so "the app's
/// second window" only means the same thing when saving and when restoring if the list
/// is put in an order that does not.
pub fn in_order(mut open: Vec<Open>) -> Vec<Open> {
    open.sort_by_key(|window| (rank(window), window.id));
    open
}

fn same_title(saved: &WindowInfo, open: &Open) -> bool {
    saved.title == open.title
}

/// On the screen and desktop the entry was saved on (a full-screen window is on a
/// desktop of its own, so there the screen is all that counts).
fn same_desktop(saved: &WindowInfo, open: &Open) -> bool {
    open.place.as_ref().is_some_and(|place| {
        saved.display.as_deref() == Some(place.display.as_str())
            && saved.fullscreen == open.fullscreen
            && (saved.fullscreen || saved.desktop.is_none_or(|number| place.holds(number)))
    })
}

type Rule = fn(&WindowInfo, &Open) -> bool;

/// Tried in turn, each on what the ones before left unpaired. Titles change (browser
/// tabs, documents), so after them comes position in the list.
const RULES: [Rule; 4] = [
    |saved, open| same_title(saved, open) && same_desktop(saved, open),
    same_title,
    same_desktop,
    |_, _| true,
];

/// A saved window and the open window taken to be the same one.
#[derive(Debug, Clone, PartialEq)]
pub struct Pair<'a> {
    /// Which entry of the profile this is.
    pub order: usize,
    pub saved: &'a WindowInfo,
    pub open: &'a Open,
}

/// Removes and returns the first unclaimed entry that satisfies `wanted`.
fn claim<'a>(
    unclaimed: &mut Vec<(usize, &'a WindowInfo)>,
    wanted: impl Fn(&WindowInfo) -> bool,
) -> Option<(usize, &'a WindowInfo)> {
    let index = unclaimed.iter().position(|(_, entry)| wanted(entry))?;
    Some(unclaimed.remove(index))
}

/// Pairs open windows with saved ones, app by app. Open windows without an entry and
/// entries without a window are left out. Pairs come in the order of `saved`.
pub fn pair<'a>(saved: &[&'a WindowInfo], open: &'a [Open]) -> Vec<Pair<'a>> {
    let mut unclaimed: Vec<(usize, &'a WindowInfo)> = saved.iter().copied().enumerate().collect();
    let mut waiting: Vec<&'a Open> = open.iter().collect();
    let mut pairs: Vec<Pair<'a>> = Vec::new();
    for rule in RULES {
        waiting.retain(|window| {
            let claimed = claim(&mut unclaimed, |entry| {
                entry.bundle_id == window.bundle_id && rule(entry, window)
            });
            pairs.extend(claimed.map(|(order, saved)| Pair {
                order,
                saved,
                open: window,
            }));
            claimed.is_none()
        });
    }
    pairs.sort_by_key(|pair| pair.order);
    pairs
}

/// What has to be done to a window, one thing at a time.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Step {
    /// Bring its desktop into view once, so that its app hands it out.
    Reach,
    /// Take it out of full screen.
    Leave,
    /// Put it on its desktop, at its frame.
    Place,
    /// Make it full screen (on its screen).
    Enter,
}

impl Pair<'_> {
    /// The screen the window was saved on, if that screen is connected.
    pub fn screen<'m>(&self, map: &'m DesktopMap) -> Option<&'m ScreenRow> {
        self.saved
            .display
            .as_deref()
            .and_then(|display| map.row(display))
    }

    /// The ordinary desktop the window belongs on. A screen that has fewer desktops now
    /// than when the profile was saved offers its last one.
    fn desktop(&self, map: &DesktopMap) -> Option<u32> {
        let row = self.screen(map)?;
        self.saved.desktop.map(|number| number.min(row.desktops()))
    }

    /// True unless the window is known to belong on another screen than it is on.
    pub fn on_screen(&self, map: &DesktopMap) -> bool {
        self.screen(map)
            .is_none_or(|row| self.open.is_on(&row.display))
    }

    fn on_desktop(&self, map: &DesktopMap) -> bool {
        self.desktop(map).is_none_or(|number| {
            self.open
                .place
                .as_ref()
                .is_some_and(|place| place.holds(number))
        })
    }

    /// The desktop that has to be in view for the window to be moved onto it:
    /// (display, position in its row). `None` when the window is on it already, or the
    /// profile does not say.
    pub fn home(&self, map: &DesktopMap) -> Option<(String, usize)> {
        let there = self.on_screen(map) && self.on_desktop(map);
        let row = self.screen(map).filter(|_| !there)?;
        self.desktop(map)
            .and_then(|number| row.row_of(number))
            .map(|index| (row.display.clone(), index))
    }

    fn needs(&self, map: &DesktopMap) -> Option<Step> {
        let placed = self.on_screen(map)
            && self.on_desktop(map)
            && self.open.frame.integral_eq(&self.saved.frame());
        match (self.saved.fullscreen, self.open.fullscreen) {
            (true, true) => (!self.on_screen(map)).then_some(Step::Leave),
            (true, false) => Some(Step::Enter),
            (false, true) => Some(Step::Leave),
            (false, false) => (!placed).then_some(Step::Place),
        }
    }

    /// The next thing to do to the window; `None` when it is as the profile has it.
    pub fn step(&self, map: &DesktopMap) -> Option<Step> {
        let needs = self.needs(map)?;
        Some(if self.open.handle.is_some() {
            needs
        } else {
            Step::Reach
        })
    }
}

#[cfg(test)]
mod tests;
