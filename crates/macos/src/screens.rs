//! Connected displays.

use crate::private::functions;
use layoutsmart_core::displays::RawDisplay;
use layoutsmart_core::geometry::Rect;
use objc2::MainThreadMarker;
use objc2_app_kit::NSScreen;
use objc2_core_foundation::CFUUID;
use objc2_core_graphics::{
    CGDisplayBounds, CGDisplayModelNumber, CGDisplaySerialNumber, CGDisplayVendorNumber,
};
use objc2_foundation::{NSNumber, ns_string};

fn display_id(screen: &NSScreen) -> Option<u32> {
    screen
        .deviceDescription()
        .objectForKey(ns_string!("NSScreenNumber"))
        .and_then(|number| number.downcast::<NSNumber>().ok())
        .map(|number| number.unsignedIntValue())
}

/// How macOS names this display in its list of desktops.
fn uuid(id: u32) -> Option<String> {
    functions()
        .display_uuid
        // SAFETY: a plain call with a display number.
        .and_then(|display_uuid| unsafe { display_uuid(id) })
        .and_then(|uuid| CFUUID::new_string(None, Some(&uuid)))
        .map(|text| text.to_string())
}

fn describe(screen: &NSScreen) -> Option<RawDisplay> {
    let id = display_id(screen)?;
    // Global coordinates with a top-left origin: the same space window positions use.
    let bounds = CGDisplayBounds(id);
    Some(RawDisplay {
        name: Some(screen.localizedName().to_string()),
        vendor: CGDisplayVendorNumber(id),
        model: CGDisplayModelNumber(id),
        serial: CGDisplaySerialNumber(id),
        uuid: uuid(id),
        bounds: Rect::new(
            bounds.origin.x,
            bounds.origin.y,
            bounds.size.width,
            bounds.size.height,
        ),
    })
}

pub fn displays() -> Vec<RawDisplay> {
    let main_thread = MainThreadMarker::new().expect("displays are read on the main thread");
    NSScreen::screens(main_thread)
        .iter()
        .filter_map(|screen| describe(&screen))
        .collect()
}
