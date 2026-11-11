//! The pointer and the keyboard. macOS shows another desktop on a screen when the
//! "move left/right a space" shortcut is pressed while the pointer is on that screen;
//! nothing else does it from outside.

use objc2_core_foundation::CGPoint;
use objc2_core_graphics::{
    CGEvent, CGEventFlags, CGEventSource, CGEventSourceStateID, CGEventTapLocation,
    CGWarpMouseCursorPosition,
};
use std::thread::sleep;
use std::time::Duration;

/// Virtual key codes of ← and →.
const ARROWS: [u16; 2] = [123, 124];

/// Between a key going down and coming up, and after it: as on a keyboard.
const KEY_GAP: Duration = Duration::from_millis(40);

pub fn pointer() -> (f64, f64) {
    let event = CGEvent::new(None);
    let location = CGEvent::location(event.as_deref());
    (location.x, location.y)
}

pub fn park_pointer(x: f64, y: f64) {
    CGWarpMouseCursorPosition(CGPoint { x, y });
}

/// An arrow key as a real keyboard sends it: Control alone is ignored; the system
/// shortcut only answers when the "fn" and "keypad" flags are there too.
fn send(source: Option<&CGEventSource>, key: u16, down: bool) {
    let event = CGEvent::new_keyboard_event(source, key, down);
    CGEvent::set_flags(
        event.as_deref(),
        CGEventFlags::MaskControl | CGEventFlags::MaskSecondaryFn | CGEventFlags::MaskNumericPad,
    );
    CGEvent::post(CGEventTapLocation::HIDEventTap, event.as_deref());
    sleep(KEY_GAP);
}

/// Presses Control+← or Control+→.
pub fn press_desktop_key(right: bool) {
    let key = ARROWS[usize::from(right)];
    let source = CGEventSource::new(CGEventSourceStateID::HIDSystemState);
    send(source.as_deref(), key, true);
    send(source.as_deref(), key, false);
}
