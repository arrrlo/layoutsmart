//! Saved-profile data model. JSON format is shared with the original Swift `winlayout`.

use crate::geometry::Rect;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DisplayInfo {
    pub name: String,
    /// `vendor-model-serial`, stable across reconnects.
    pub id: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl DisplayInfo {
    pub fn frame(&self) -> Rect {
        Rect::new(self.x, self.y, self.width, self.height)
    }
}

fn is_false(value: &bool) -> bool {
    !*value
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowInfo {
    pub app: String,
    pub bundle_id: String,
    pub title: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    /// The screen the window was on (`DisplayInfo::id`), when macOS told.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display: Option<String>,
    /// Its desktop on that screen, counted from 1 among the ordinary desktops.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub desktop: Option<u32>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub fullscreen: bool,
}

impl WindowInfo {
    pub fn new(app: &str, bundle_id: &str, title: &str, frame: Rect) -> Self {
        Self {
            app: app.into(),
            bundle_id: bundle_id.into(),
            title: title.into(),
            x: frame.x,
            y: frame.y,
            width: frame.width,
            height: frame.height,
            display: None,
            desktop: None,
            fullscreen: false,
        }
    }

    /// The same window, recorded as being on ordinary desktop `number` of `display`.
    pub fn on_desktop(mut self, display: &str, number: u32) -> Self {
        self.display = Some(display.into());
        self.desktop = Some(number);
        self
    }

    /// The same window, recorded as on `display` without a desktop of its own (a window
    /// that shows on all desktops).
    pub fn on_display(mut self, display: &str) -> Self {
        self.display = Some(display.into());
        self
    }

    /// The same window, recorded as full screen on `display`.
    pub fn in_full_screen(mut self, display: &str) -> Self {
        self.display = Some(display.into());
        self.desktop = None;
        self.fullscreen = true;
        self
    }

    pub fn frame(&self) -> Rect {
        Rect::new(self.x, self.y, self.width, self.height)
    }
}

/// What a screen was showing when a profile was saved.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Showing {
    /// An ordinary desktop, by number.
    Desktop(u32),
    /// A full-screen app's window.
    #[serde(rename_all = "camelCase")]
    FullScreen {
        bundle_id: String,
        title: String,
    },
    Unknown,
}

/// One screen's desktops at the time a profile was saved.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedScreen {
    /// `DisplayInfo::id`.
    pub display: String,
    /// How many ordinary desktops the screen had.
    pub desktops: u32,
    pub showing: Showing,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Layout {
    pub profile: String,
    pub saved_at: String,
    pub displays: Vec<DisplayInfo>,
    pub windows: Vec<WindowInfo>,
    /// Empty for profiles saved before desktops were recorded (or when macOS did not tell).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub desktops: Vec<SavedScreen>,
}

impl Layout {
    pub fn from_json(json: &str) -> Result<Layout, serde_json::Error> {
        serde_json::from_str(json)
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("Layout always serializes")
    }

    pub fn knows_desktops(&self) -> bool {
        !self.desktops.is_empty()
    }

    /// What was recorded about the screen with this identity.
    pub fn screen(&self, display: &str) -> Option<&SavedScreen> {
        self.desktops
            .iter()
            .find(|screen| screen.display == display)
    }
}

#[cfg(test)]
mod tests;
