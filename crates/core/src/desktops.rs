//! Desktops (Spaces): which desktop of which screen a window is on.
//!
//! macOS gives every desktop an id that means nothing outside the running session. This
//! module turns those ids into something that can be saved and shown: "screen X, desktop 2"
//! or "screen X, full screen".

use crate::displays::{RawDisplay, display_identity};

/// One desktop as macOS reports it.
#[derive(Debug, Clone, PartialEq)]
pub struct RawSpace {
    pub id: u64,
    /// True for the private desktop of a full-screen app.
    pub fullscreen: bool,
}

/// The desktops of one screen, in Mission Control order.
#[derive(Debug, Clone, PartialEq)]
pub struct RawScreen {
    /// Identifies the display (the same text as `RawDisplay::uuid`).
    pub uuid: String,
    /// The desktop that screen is showing.
    pub current: u64,
    pub spaces: Vec<RawSpace>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Kind {
    /// An ordinary desktop, numbered from 1 among the ordinary desktops of its screen.
    Desktop(u32),
    FullScreen,
}

impl Kind {
    pub fn label(&self) -> String {
        match self {
            Kind::Desktop(number) => format!("desktop {number}"),
            Kind::FullScreen => "full screen".to_string(),
        }
    }

    pub fn is_fullscreen(&self) -> bool {
        *self == Kind::FullScreen
    }
}

/// Where a window is.
#[derive(Debug, Clone, PartialEq)]
pub struct Place {
    /// The display's identity (`DisplayInfo::id`).
    pub display: String,
    pub display_name: String,
    pub kind: Kind,
    /// Position in the screen's row of desktops, from 0.
    pub row: usize,
    /// True if that desktop is the one the screen is showing.
    pub showing: bool,
}

impl Place {
    /// `Main · desktop 2`, `Side · full screen (hidden)`.
    pub fn label(&self) -> String {
        let hidden = if self.showing { "" } else { " (hidden)" };
        format!("{} · {}{hidden}", self.display_name, self.kind.label())
    }
}

/// One screen's desktops, for listing.
#[derive(Debug, Clone, PartialEq)]
pub struct ScreenRow {
    pub display: String,
    pub name: String,
    pub kinds: Vec<Kind>,
    /// Index into `kinds` of the desktop in view.
    pub showing: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct DesktopMap {
    rows: Vec<ScreenRow>,
    places: Vec<(u64, Place)>,
}

/// (identity, name) of the display with this identifier; the identifier itself if none has it.
fn identify(displays: &[RawDisplay], uuid: &str) -> (String, String) {
    displays
        .iter()
        .enumerate()
        .find(|(_, display)| display.uuid.as_deref() == Some(uuid))
        .map(|(index, display)| {
            (
                display_identity(display.vendor, display.model, display.serial),
                display
                    .name
                    .clone()
                    .unwrap_or_else(|| format!("Display {}", index + 1)),
            )
        })
        .unwrap_or_else(|| (uuid.to_string(), uuid.to_string()))
}

/// Numbers the ordinary desktops 1, 2, 3… and leaves full-screen ones unnumbered.
fn kinds(screen: &RawScreen) -> Vec<Kind> {
    let mut number = 0;
    screen
        .spaces
        .iter()
        .map(|space| {
            if space.fullscreen {
                Kind::FullScreen
            } else {
                number += 1;
                Kind::Desktop(number)
            }
        })
        .collect()
}

impl DesktopMap {
    pub fn new(displays: &[RawDisplay], screens: &[RawScreen]) -> DesktopMap {
        let mut map = DesktopMap::default();
        for screen in screens {
            let (display, name) = identify(displays, &screen.uuid);
            let kinds = kinds(screen);
            let showing = screen
                .spaces
                .iter()
                .position(|space| space.id == screen.current);
            for (row, (space, kind)) in screen.spaces.iter().zip(&kinds).enumerate() {
                let place = Place {
                    display: display.clone(),
                    display_name: name.clone(),
                    kind: *kind,
                    row,
                    showing: showing == Some(row),
                };
                map.places.push((space.id, place));
            }
            map.rows.push(ScreenRow {
                display,
                name,
                kinds,
                showing,
            });
        }
        map
    }

    /// False when macOS told us nothing about desktops (then every window's place is unknown).
    pub fn is_known(&self) -> bool {
        !self.rows.is_empty()
    }

    pub fn rows(&self) -> &[ScreenRow] {
        &self.rows
    }

    pub fn row(&self, display: &str) -> Option<&ScreenRow> {
        self.rows.iter().find(|row| row.display == display)
    }

    /// The place of a window that is on these desktops (the first known one decides).
    pub fn place(&self, spaces: &[u64]) -> Option<Place> {
        spaces
            .iter()
            .find_map(|id| self.places.iter().find(|(known, _)| known == id))
            .map(|(_, place)| place.clone())
    }
}

#[cfg(test)]
mod tests;
