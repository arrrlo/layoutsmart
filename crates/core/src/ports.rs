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

/// A question answered by typing a name.
#[derive(Debug, Clone, PartialEq)]
pub struct TextPrompt {
    pub title: String,
    pub message: String,
    /// Text already in the box.
    pub value: String,
    /// Label of the button that accepts (the other button is always Cancel).
    pub confirm: String,
}

/// A yes/no question.
#[derive(Debug, Clone, PartialEq)]
pub struct Question {
    pub title: String,
    pub message: String,
    pub confirm: String,
    /// True when confirming throws something away.
    pub destructive: bool,
}

/// The dialogs the app can show. The macOS layer displays them; it decides nothing.
pub trait Ui {
    /// The typed text, or `None` if the user cancelled.
    fn ask_text(&self, prompt: &TextPrompt) -> Option<String>;
    fn confirm(&self, question: &Question) -> bool;
    fn alert(&self, title: &str, message: &str);
    fn open_accessibility_settings(&self);
}

pub trait LoginItem {
    /// Asks macOS to start the app at login. True if it agreed.
    fn register(&self) -> bool;
}

/// Things only the running app can do.
pub trait Host {
    fn open_dashboard(&self);
    fn quit(&self);
}
