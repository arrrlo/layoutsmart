//! What the core needs from the outside world. The macOS layer implements these;
//! tests use fakes.

use crate::desktops::RawScreen;
use crate::displays::RawDisplay;
use crate::geometry::Rect;
use crate::windows::{RawWindow, ServerWindow};

pub trait WindowSystem {
    /// Whether macOS lets this process see and move other apps' windows (Accessibility).
    /// With `prompt`, macOS may show its own permission dialog.
    fn is_trusted(&self, prompt: bool) -> bool;
    fn displays(&self) -> Vec<RawDisplay>;
    /// The windows every regular app lists, exactly as reported. macOS lists a window only
    /// while its desktop is in view.
    fn windows(&self) -> Vec<RawWindow>;
    /// Looks for these windows — `(process id, window-server number)` — by addressing
    /// their apps' elements by number, which is how a window on a desktop that is not in
    /// view can be reached. Costly (up to a couple of thousand small requests per app), so
    /// asked only for windows known to be missing. Returns the ones found.
    fn numbered_windows(&self, wanted: &[(i32, u32)]) -> Vec<RawWindow>;
    /// Every window the window server lists for regular apps (all desktops; frames only).
    fn server_windows(&self) -> Vec<ServerWindow>;
    /// The desktops of each screen. Empty when macOS does not tell.
    fn screens(&self) -> Vec<RawScreen>;
    /// Moves and resizes the window with this `handle` (from `windows()` or `numbered_windows()`).
    fn set_frame(&self, handle: usize, frame: Rect) -> bool;
    /// Where that window is now — to check that a move took effect.
    fn frame(&self, handle: usize) -> Option<Rect>;
    /// Asks the window to enter or leave full screen. macOS animates the change, so it is
    /// complete only some time after this returns.
    fn set_fullscreen(&self, handle: usize, on: bool) -> bool;

    /// The system shortcuts "move left a space" and "move right a space", as configured.
    fn desktop_shortcuts(&self) -> [Shortcut; 2];
    /// Sends the "one desktop to the left/right" shortcut. macOS applies it to the screen
    /// the pointer is on.
    fn press_desktop_key(&self, right: bool);
    fn pointer(&self) -> (f64, f64);
    fn park_pointer(&self, x: f64, y: f64);
    /// Waits, to let macOS finish an animation.
    fn pause(&self, milliseconds: u64);
    /// Process id of the app in front.
    fn front_app(&self) -> Option<i32>;
    /// Brings that app to the front.
    fn activate(&self, pid: i32);
}

/// One system keyboard shortcut as found in the user's settings. `None` means the
/// setting is absent, which is how macOS stores "left at its default".
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Shortcut {
    pub enabled: Option<bool>,
    /// Virtual key code, if the user chose a different key.
    pub key: Option<i64>,
    /// Modifier flags, if the user chose different ones.
    pub modifiers: Option<i64>,
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
