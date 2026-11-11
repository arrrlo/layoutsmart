//! The user's keyboard shortcuts for moving between desktops, as System Settings
//! stores them. The core decides whether they are usable.

use crate::plist::{self, Dictionary};
use layoutsmart_core::ports::Shortcut;
use objc2::AnyThread;
use objc2::rc::Retained;
use objc2_foundation::{NSArray, NSNumber, NSString, NSUserDefaults};

/// macOS's numbers for "Move left a space" and "Move right a space".
const MOVE_LEFT: &str = "79";
const MOVE_RIGHT: &str = "81";

fn all() -> Option<Retained<Dictionary>> {
    let domain = NSString::from_str("com.apple.symbolichotkeys");
    NSUserDefaults::initWithSuiteName(NSUserDefaults::alloc(), Some(&domain))
        .and_then(|defaults| defaults.dictionaryForKey(&NSString::from_str("AppleSymbolicHotKeys")))
}

/// A shortcut's parameters are (character, key code, modifier flags).
fn parameter(parameters: &NSArray, index: usize) -> Option<i64> {
    parameters
        .iter()
        .nth(index)
        .and_then(|item| item.downcast::<NSNumber>().ok())
        .map(|number| number.longLongValue())
}

fn shortcut(all: Option<&Dictionary>, number: &str) -> Shortcut {
    let entry = all.and_then(|all| plist::dictionary(all, number));
    let parameters = entry
        .as_deref()
        .and_then(|entry| plist::dictionary(entry, "value"))
        .and_then(|value| plist::array(&value, "parameters"));
    Shortcut {
        enabled: entry
            .as_deref()
            .and_then(|entry| plist::signed(entry, "enabled"))
            .map(|flag| flag != 0),
        key: parameters.as_deref().and_then(|list| parameter(list, 1)),
        modifiers: parameters.as_deref().and_then(|list| parameter(list, 2)),
    }
}

/// (move left a space, move right a space)
pub fn desktop_shortcuts() -> [Shortcut; 2] {
    let all = all();
    [
        shortcut(all.as_deref(), MOVE_LEFT),
        shortcut(all.as_deref(), MOVE_RIGHT),
    ]
}
