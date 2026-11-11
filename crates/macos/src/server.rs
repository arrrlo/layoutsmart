//! What the window server lists: every window on every desktop, with its frame only.

use crate::apps::App;
use crate::plist::{self, Dictionary};
use crate::spaces;
use layoutsmart_core::geometry::Rect;
use layoutsmart_core::windows::ServerWindow;
use objc2::runtime::AnyObject;
use objc2_core_foundation::{CFArray, CFRetained};
use objc2_core_graphics::{CGWindowListCopyWindowInfo, CGWindowListOption, kCGNullWindowID};
use objc2_foundation::NSArray;

fn bounds(window: &Dictionary) -> Rect {
    let part = |key: &str| {
        plist::dictionary(window, "kCGWindowBounds")
            .and_then(|bounds| plist::float(&bounds, key))
            .unwrap_or_default()
    };
    Rect::new(part("X"), part("Y"), part("Width"), part("Height"))
}

/// Layer 0 holds ordinary windows; menus, tooltips and overlays are on other layers and
/// are never looked up further.
const WINDOW_LAYER: i64 = 0;

/// Asking where a window is costs a round trip, and most entries are not windows.
fn spaces_of(window_id: u32, layer: i64) -> Vec<u64> {
    if layer == WINDOW_LAYER {
        spaces::of_window(window_id)
    } else {
        Vec::new()
    }
}

fn describe(window: &Dictionary, app: &App, window_id: u64) -> ServerWindow {
    let layer = plist::signed(window, "kCGWindowLayer").unwrap_or(-1);
    ServerWindow {
        app: app.name.clone(),
        bundle_id: app.bundle_id.clone(),
        pid: app.pid,
        window_id: window_id as u32,
        layer: layer as i32,
        alpha: plist::float(window, "kCGWindowAlpha").unwrap_or_default(),
        bounds: bounds(window),
        spaces: spaces_of(window_id as u32, layer),
    }
}

/// One entry of the list, if it belongs to one of `apps`.
fn window(window: &Dictionary, apps: &[App]) -> Option<ServerWindow> {
    let pid = plist::signed(window, "kCGWindowOwnerPID").unwrap_or(-1);
    apps.iter()
        .find(|app| i64::from(app.pid) == pid)
        .zip(plist::unsigned(window, "kCGWindowNumber"))
        .map(|(app, window_id)| describe(window, app, window_id))
}

/// Every window of these apps that the window server knows, on any desktop.
pub fn windows(apps: &[App]) -> Vec<ServerWindow> {
    CGWindowListCopyWindowInfo(CGWindowListOption::OptionAll, kCGNullWindowID)
        .map(|list| {
            // SAFETY: the list holds objects (dictionaries); each one is checked on use.
            let list = unsafe { CFRetained::cast_unchecked::<CFArray<AnyObject>>(list) };
            let list: &NSArray = (*list).as_ref();
            plist::dictionaries(list)
        })
        .unwrap_or_default()
        .iter()
        .filter_map(|entry| window(entry, apps))
        .collect()
}
