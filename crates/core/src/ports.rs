//! What the core needs from the outside world. The macOS layer implements these;
//! tests use fakes.

use crate::displays::RawDisplay;
use crate::geometry::Rect;
use crate::windows::RawWindow;

pub trait WindowSystem {
    /// Whether macOS lets this process see and move other apps' windows (Accessibility).
    /// With `prompt`, macOS may show its own permission dialog.
    fn is_trusted(&self, prompt: bool) -> bool;
    fn displays(&self) -> Vec<RawDisplay>;
    /// Every window of every regular app, exactly as reported.
    fn windows(&self) -> Vec<RawWindow>;
    /// Moves and resizes the window at index `handle` of the last `windows()` result.
    fn set_frame(&self, handle: usize, frame: Rect) -> bool;
}

pub trait Clock {
    /// Seconds since 1970-01-01T00:00:00Z.
    fn unix_seconds(&self) -> u64;
}
