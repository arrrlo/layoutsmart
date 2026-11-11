//! The dashboard window's state, and the picture it draws of a profile.

use crate::flows::{self, Session};
use crate::geometry::{Rect, Size};
use crate::model::{DisplayInfo, Layout, SavedScreen, Showing, WindowInfo};
use crate::store::ProfileStore;
use crate::text::plural;
use crate::timefmt;

/// Empty space kept around the picture, in points.
pub const PADDING: f64 = 20.0;
/// How many window colours the macOS layer provides.
pub const PALETTE_SIZE: usize = 8;

pub const NO_PROFILES: &str =
    "No saved profiles yet.\nChoose “Save Window Positions…” from the menu bar icon.";
pub const NOTHING_SELECTED: &str = "Select a profile to see its screens and windows.";

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ShapeKind {
    Display = 0,
    Window = 1,
}

/// One rectangle to draw, in canvas coordinates (top-left origin).
#[derive(Debug, Clone, PartialEq)]
pub struct Shape {
    pub kind: ShapeKind,
    pub rect: Rect,
    pub label: String,
    pub detail: String,
    /// Index into the window palette (always 0 for displays).
    pub color: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Scene {
    pub shapes: Vec<Shape>,
    /// Shown instead of a picture when there is nothing to draw.
    pub message: String,
}

impl Scene {
    fn saying(message: &str) -> Scene {
        Scene {
            shapes: Vec::new(),
            message: message.to_string(),
        }
    }
}

/// Maps screen coordinates onto the canvas: one scale for both axes, centred.
struct Fit {
    scale: f64,
    dx: f64,
    dy: f64,
}

impl Fit {
    fn new(bounds: Rect, canvas: Size) -> Option<Fit> {
        let room = Size::new(canvas.width - 2.0 * PADDING, canvas.height - 2.0 * PADDING);
        if bounds.width <= 0.0 || bounds.height <= 0.0 || room.width <= 0.0 || room.height <= 0.0 {
            return None;
        }
        let scale = (room.width / bounds.width).min(room.height / bounds.height);
        Some(Fit {
            scale,
            dx: (canvas.width - bounds.width * scale) / 2.0 - bounds.x * scale,
            dy: (canvas.height - bounds.height * scale) / 2.0 - bounds.y * scale,
        })
    }

    fn apply(&self, rect: Rect) -> Rect {
        Rect::new(
            self.dx + rect.x * self.scale,
            self.dy + rect.y * self.scale,
            rect.width * self.scale,
            rect.height * self.scale,
        )
    }
}

/// The part of `window` on the screen that holds most of it.
fn visible_part(window: Rect, screens: &[Rect]) -> Option<Rect> {
    screens
        .iter()
        .filter_map(|screen| screen.intersection(&window))
        .max_by(|a, b| (a.width * a.height).total_cmp(&(b.width * b.height)))
}

fn app_order(layout: &Layout) -> Vec<&str> {
    let mut apps: Vec<&str> = Vec::new();
    for window in &layout.windows {
        if !apps.contains(&window.bundle_id.as_str()) {
            apps.push(&window.bundle_id);
        }
    }
    apps
}

fn color_of(apps: &[&str], bundle_id: &str) -> usize {
    apps.iter().position(|app| *app == bundle_id).unwrap_or(0) % PALETTE_SIZE
}

/// One screen's row of tabs: its ordinary desktops, then its full-screen windows.
#[derive(Debug, Clone, PartialEq)]
pub struct TabRow {
    /// The screen (`DisplayInfo::id`).
    pub display: String,
    pub name: String,
    pub tabs: Vec<String>,
    /// How many of the tabs, from the left, are ordinary desktops.
    pub desktops: usize,
    pub selected: usize,
}

/// The full-screen windows saved on a screen, in the order of their desktops.
fn fullscreen_on<'a>(layout: &'a Layout, display: &str) -> Vec<&'a WindowInfo> {
    layout
        .windows
        .iter()
        .filter(|window| window.fullscreen && window.display.as_deref() == Some(display))
        .collect()
}

/// The tab a screen opens on: what it showed when the profile was saved.
fn opening_tab(saved: &SavedScreen, full: &[&WindowInfo]) -> usize {
    match &saved.showing {
        Showing::Desktop(number) => (*number).min(saved.desktops).saturating_sub(1) as usize,
        Showing::FullScreen { bundle_id, title } => full
            .iter()
            .position(|window| window.bundle_id == *bundle_id && window.title == *title)
            .or_else(|| {
                full.iter()
                    .position(|window| window.bundle_id == *bundle_id)
            })
            .map_or(0, |index| saved.desktops as usize + index),
        Showing::Unknown => 0,
    }
}

fn tab_row(layout: &Layout, display: &DisplayInfo, chosen: &[(String, usize)]) -> Option<TabRow> {
    let saved = layout.screen(&display.id)?;
    let full = fullscreen_on(layout, &display.id);
    let tabs: Vec<String> = (1..=saved.desktops)
        .map(|number| format!("Desktop {number}"))
        .chain(full.iter().map(|window| window.app.clone()))
        .collect();
    let picked = chosen
        .iter()
        .find(|(id, _)| *id == display.id)
        .map(|(_, tab)| *tab);
    let selected = picked
        .unwrap_or_else(|| opening_tab(saved, &full))
        .min(tabs.len().saturating_sub(1));
    Some(TabRow {
        display: display.id.clone(),
        name: display.name.clone(),
        tabs,
        desktops: saved.desktops as usize,
        selected,
    })
}

/// A row of tabs for each screen whose desktops the profile recorded. `chosen` holds the
/// tabs the user picked, by screen; the others open on what the screen showed.
pub fn tab_rows(layout: &Layout, chosen: &[(String, usize)]) -> Vec<TabRow> {
    layout
        .displays
        .iter()
        .filter_map(|display| tab_row(layout, display, chosen))
        .filter(|row| !row.tabs.is_empty())
        .collect()
}

/// Whether a window is on the desktop (or is the full-screen window) its screen's row
/// has selected. A window on all desktops is on every ordinary one.
fn on_tab(window: &WindowInfo, layout: &Layout, row: &TabRow) -> bool {
    if window.fullscreen {
        fullscreen_on(layout, &row.display)
            .get(row.selected.wrapping_sub(row.desktops))
            .is_some_and(|shown| std::ptr::eq(*shown, window))
    } else {
        row.selected < row.desktops
            && window
                .desktop
                .is_none_or(|number| number as usize == row.selected + 1)
    }
}

/// Windows whose screen has no row of tabs (nothing is known about its desktops) are
/// always drawn.
fn is_shown(window: &WindowInfo, layout: &Layout, rows: &[TabRow]) -> bool {
    window
        .display
        .as_deref()
        .and_then(|display| rows.iter().find(|row| row.display == display))
        .is_none_or(|row| on_tab(window, layout, row))
}

/// The screens of `layout`, then the windows on the desktops selected in `rows`, from
/// back to front, scaled to `canvas`.
pub fn shapes(layout: &Layout, canvas: Size, rows: &[TabRow]) -> Vec<Shape> {
    let screens: Vec<Rect> = layout.displays.iter().map(DisplayInfo::frame).collect();
    let fit = Rect::union_all(screens.iter().copied()).and_then(|bounds| Fit::new(bounds, canvas));
    let Some(fit) = fit else {
        return Vec::new();
    };
    let apps = app_order(layout);
    let displays = layout.displays.iter().map(|display| Shape {
        kind: ShapeKind::Display,
        rect: fit.apply(display.frame()),
        label: display.name.clone(),
        detail: format!("{:.0} × {:.0}", display.width, display.height),
        color: 0,
    });
    let windows = layout
        .windows
        .iter()
        .rev()
        .filter(|window| is_shown(window, layout, rows))
        .filter_map(|window| {
            Some(Shape {
                kind: ShapeKind::Window,
                rect: fit.apply(visible_part(window.frame(), &screens)?),
                label: window.app.clone(),
                detail: window.title.clone(),
                color: color_of(&apps, &window.bundle_id),
            })
        });
    displays.chain(windows).collect()
}

/// The profile list on the left of the dashboard and which row is selected.
#[derive(Debug, Clone, PartialEq)]
pub struct Dashboard {
    names: Vec<String>,
    selected: Option<usize>,
    /// The tabs the user picked for the selected profile: (screen, tab).
    chosen: Vec<(String, usize)>,
}

impl Dashboard {
    pub fn open(store: &ProfileStore) -> Dashboard {
        let names = store.names();
        Dashboard {
            selected: (!names.is_empty()).then_some(0),
            names,
            chosen: Vec::new(),
        }
    }

    pub fn rows(&self) -> usize {
        self.names.len()
    }

    pub fn row_title(&self, row: usize) -> String {
        self.names.get(row).cloned().unwrap_or_default()
    }

    pub fn selected_row(&self) -> Option<usize> {
        self.selected
    }

    pub fn selected_name(&self) -> Option<&str> {
        self.selected
            .and_then(|row| self.names.get(row))
            .map(String::as_str)
    }

    /// Rename and Delete only make sense with a profile selected.
    pub fn can_edit(&self) -> bool {
        self.selected.is_some()
    }

    /// Selecting another profile forgets the tabs picked for this one.
    pub fn select(&mut self, row: Option<usize>) {
        let row = row.filter(|row| *row < self.names.len());
        if row != self.selected {
            self.chosen.clear();
        }
        self.selected = row;
    }

    /// Re-reads the list. Selects `prefer` if it exists; otherwise stays on the same
    /// profile, or — if that is gone — on the same row.
    pub fn refresh(&mut self, store: &ProfileStore, prefer: Option<&str>) {
        let current = self.selected_name().map(String::from);
        let row = self.selected;
        self.names = store.names();
        let find = |name: &str| self.names.iter().position(|n| n == name);
        let last = self.names.len().checked_sub(1);
        self.selected = prefer
            .and_then(find)
            .or_else(|| current.as_deref().and_then(find))
            .or(row)
            .and_then(|row| last.map(|last| row.min(last)));
        if self.selected_name() != current.as_deref() {
            self.chosen.clear();
        }
    }

    fn layout(&self, store: &ProfileStore) -> Option<Layout> {
        self.selected_name().and_then(|name| store.load(name).ok())
    }

    /// The rows of desktop tabs for the selected profile.
    pub fn tab_rows(&self, store: &ProfileStore) -> Vec<TabRow> {
        self.layout(store)
            .map(|layout| tab_rows(&layout, &self.chosen))
            .unwrap_or_default()
    }

    /// A click on tab `tab` of row `row`.
    pub fn choose(&mut self, store: &ProfileStore, row: usize, tab: usize) {
        let display = self
            .tab_rows(store)
            .get(row)
            .filter(|row| tab < row.tabs.len())
            .map(|row| row.display.clone());
        if let Some(display) = display {
            self.chosen.retain(|(id, _)| *id != display);
            self.chosen.push((display, tab));
        }
    }

    /// One line about the selected profile, e.g. `2 displays · 14 windows · saved …`.
    pub fn summary(&self, store: &ProfileStore) -> String {
        match self.selected_name().map(|name| store.load(name)) {
            None => String::new(),
            Some(Err(error)) => error.to_string(),
            Some(Ok(layout)) => format!(
                "{} · {} · saved {}",
                plural(layout.displays.len(), "display"),
                plural(layout.windows.len(), "window"),
                timefmt::display(&layout.saved_at)
            ),
        }
    }

    pub fn scene(&self, store: &ProfileStore, canvas: Size) -> Scene {
        match self.selected_name().map(|name| store.load(name)) {
            None if self.names.is_empty() => Scene::saying(NO_PROFILES),
            None => Scene::saying(NOTHING_SELECTED),
            Some(Err(error)) => Scene::saying(&error.to_string()),
            Some(Ok(layout)) => Scene {
                shapes: shapes(&layout, canvas, &tab_rows(&layout, &self.chosen)),
                message: String::new(),
            },
        }
    }

    pub fn rename_selected(&mut self, session: &Session) {
        let renamed = self
            .selected_name()
            .and_then(|old| flows::rename_flow(session, old));
        self.refresh(session.backend.store, renamed.as_deref());
    }

    pub fn delete_selected(&mut self, session: &Session) {
        if let Some(name) = self.selected_name() {
            flows::delete_flow(session, name);
        }
        self.refresh(session.backend.store, None);
    }
}

#[cfg(test)]
mod tests;
