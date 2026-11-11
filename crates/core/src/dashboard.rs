//! The dashboard window's state, and the picture it draws of a profile.

use crate::flows::{self, Session};
use crate::geometry::{Rect, Size};
use crate::model::{DisplayInfo, Layout};
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

/// The screens of `layout`, then its windows from back to front, scaled to `canvas`.
pub fn shapes(layout: &Layout, canvas: Size) -> Vec<Shape> {
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
    let windows = layout.windows.iter().rev().filter_map(|window| {
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
pub struct Dashboard {
    names: Vec<String>,
    selected: Option<usize>,
}

impl Dashboard {
    pub fn open(store: &ProfileStore) -> Dashboard {
        let names = store.names();
        Dashboard {
            selected: (!names.is_empty()).then_some(0),
            names,
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

    pub fn select(&mut self, row: Option<usize>) {
        self.selected = row.filter(|row| *row < self.names.len());
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
                shapes: shapes(&layout, canvas),
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
