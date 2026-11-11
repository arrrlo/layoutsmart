//! Decides which of the windows macOS reports are ones Layoutsmart manages.

use crate::geometry::Rect;

/// Accessibility subrole of an ordinary document/app window.
pub const STANDARD_WINDOW: &str = "AXStandardWindow";

/// One window exactly as the Accessibility API reports it. Any field may be missing.
#[derive(Debug, Clone, PartialEq)]
pub struct RawWindow {
    pub app: Option<String>,
    pub bundle_id: Option<String>,
    pub pid: i32,
    pub subrole: Option<String>,
    pub minimized: Option<bool>,
    pub fullscreen: Option<bool>,
    pub title: Option<String>,
    pub position: Option<(f64, f64)>,
    pub size: Option<(f64, f64)>,
}

/// A window Layoutsmart can save and move. `handle` is its index in the raw list.
#[derive(Debug, Clone, PartialEq)]
pub struct LiveWindow {
    pub handle: usize,
    pub app: String,
    pub bundle_id: String,
    pub title: String,
    pub frame: Rect,
}

impl RawWindow {
    fn is_managed(&self) -> bool {
        self.subrole.as_deref() == Some(STANDARD_WINDOW)
            && self.minimized != Some(true)
            && self.fullscreen != Some(true)
    }

    fn frame(&self) -> Option<Rect> {
        let (x, y) = self.position?;
        let (width, height) = self.size?;
        Some(Rect::new(x, y, width, height))
    }

    fn to_live(&self, handle: usize) -> Option<LiveWindow> {
        let frame = self.frame()?;
        let app = self
            .app
            .clone()
            .unwrap_or_else(|| format!("pid {}", self.pid));
        Some(LiveWindow {
            handle,
            bundle_id: self.bundle_id.clone().unwrap_or_else(|| app.clone()),
            app,
            title: self.title.clone().unwrap_or_default(),
            frame,
        })
    }
}

/// Standard, visible, non-fullscreen windows with a known frame.
pub fn live_windows(raw: &[RawWindow]) -> Vec<LiveWindow> {
    raw.iter()
        .enumerate()
        .filter(|(_, w)| w.is_managed())
        .filter_map(|(i, w)| w.to_live(i))
        .collect()
}

#[cfg(test)]
mod tests;
