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
        }
    }

    pub fn frame(&self) -> Rect {
        Rect::new(self.x, self.y, self.width, self.height)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Layout {
    pub profile: String,
    pub saved_at: String,
    pub displays: Vec<DisplayInfo>,
    pub windows: Vec<WindowInfo>,
}

impl Layout {
    pub fn from_json(json: &str) -> Result<Layout, serde_json::Error> {
        serde_json::from_str(json)
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("Layout always serializes")
    }
}

#[cfg(test)]
mod tests;
