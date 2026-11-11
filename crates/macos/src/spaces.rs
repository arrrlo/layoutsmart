//! Desktops (Spaces): which exist on which screen, and which a window is on.
//! Uses undocumented functions; returns nothing when they are unavailable.

use crate::plist::{self, Dictionary};
use crate::private::functions;
use layoutsmart_core::desktops::{RawScreen, RawSpace};
use objc2_foundation::{NSArray, NSNumber};

/// macOS's code for the private desktop of a full-screen app.
const FULLSCREEN_SPACE: u64 = 4;
/// "Every kind of desktop" when asking where a window is.
const ALL_SPACES: i32 = 7;

fn space(space: &Dictionary) -> Option<RawSpace> {
    plist::unsigned(space, "ManagedSpaceID").map(|id| RawSpace {
        id,
        fullscreen: plist::unsigned(space, "type") == Some(FULLSCREEN_SPACE),
    })
}

fn spaces(screen: &Dictionary) -> Vec<RawSpace> {
    plist::array(screen, "Spaces")
        .map(|list| plist::dictionaries(&list))
        .unwrap_or_default()
        .iter()
        .filter_map(|entry| space(entry))
        .collect()
}

fn current(screen: &Dictionary) -> u64 {
    plist::dictionary(screen, "Current Space")
        .and_then(|space| plist::unsigned(&space, "ManagedSpaceID"))
        .unwrap_or_default()
}

fn screen(screen: &Dictionary) -> Option<RawScreen> {
    plist::string(screen, "Display Identifier").map(|uuid| RawScreen {
        uuid,
        current: current(screen),
        spaces: spaces(screen),
    })
}

/// Every screen's row of desktops.
pub fn screens() -> Vec<RawScreen> {
    let functions = functions();
    functions
        .connection
        .zip(functions.display_spaces)
        // SAFETY: plain calls; the result is a list we own, read defensively below.
        .and_then(|(connection, display_spaces)| unsafe { display_spaces(connection()) })
        .map(|list| plist::dictionaries(&list))
        .unwrap_or_default()
        .iter()
        .filter_map(|entry| screen(entry))
        .collect()
}

fn ids(list: &NSArray) -> Vec<u64> {
    list.iter()
        .filter_map(|item| item.downcast::<NSNumber>().ok())
        .map(|number| number.unsignedLongLongValue())
        .collect()
}

/// The desktops a window is on.
pub fn of_window(window_id: u32) -> Vec<u64> {
    let functions = functions();
    let windows = NSArray::from_retained_slice(&[NSNumber::new_u32(window_id)]);
    functions
        .connection
        .zip(functions.spaces_for_windows)
        // SAFETY: plain calls with a list of window numbers, as the function expects.
        .and_then(|(connection, spaces_for_windows)| unsafe {
            spaces_for_windows(connection(), ALL_SPACES, &windows)
        })
        .map(|list| ids(&list))
        .unwrap_or_default()
}
