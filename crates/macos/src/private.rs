//! Undocumented macOS functions, looked up by name the first time they are needed.
//!
//! Nothing here is linked against: a function that a macOS version does not have is simply
//! `None`, and whatever is built on it switches itself off. That is what keeps Layoutsmart
//! working (as version 0.1 did) if Apple removes one of them.

use objc2::rc::Retained;
use objc2_application_services::{AXError, AXUIElement};
use objc2_core_foundation::{CFData, CFRetained, CFUUID};
use objc2_foundation::{NSArray, NSNumber};
use std::ffi::{CStr, c_void};
use std::ptr::NonNull;
use std::sync::OnceLock;

/// An accessibility element for a token naming an app and an element number.
pub type ElementForToken = unsafe extern "C-unwind" fn(&CFData) -> Option<CFRetained<AXUIElement>>;
/// The window-server number of the window an element belongs to.
pub type WindowOfElement = unsafe extern "C-unwind" fn(&AXUIElement, *mut u32) -> AXError;
/// This process's connection to the window server.
pub type Connection = unsafe extern "C-unwind" fn() -> i32;
/// Every screen's desktops (a list of dictionaries).
pub type DisplaySpaces = unsafe extern "C-unwind" fn(i32) -> Option<Retained<NSArray>>;
/// The desktops the given windows are on.
pub type SpacesForWindows =
    unsafe extern "C-unwind" fn(i32, i32, &NSArray<NSNumber>) -> Option<Retained<NSArray>>;
/// The identifier macOS uses for a display in its list of desktops. (Public, but in a
/// framework not otherwise linked.)
pub type DisplayUuid = unsafe extern "C-unwind" fn(u32) -> Option<CFRetained<CFUUID>>;

pub struct Functions {
    pub element_for_token: Option<ElementForToken>,
    pub window_of_element: Option<WindowOfElement>,
    pub connection: Option<Connection>,
    pub display_spaces: Option<DisplaySpaces>,
    pub spaces_for_windows: Option<SpacesForWindows>,
    pub display_uuid: Option<DisplayUuid>,
}

fn address(name: &CStr) -> Option<NonNull<c_void>> {
    // SAFETY: looking a name up among the already loaded libraries has no preconditions.
    NonNull::new(unsafe { libc::dlsym(libc::RTLD_DEFAULT, name.as_ptr()) })
}

/// SAFETY: `F` must be the function-pointer type of the function with this name.
unsafe fn function<F: Copy>(name: &CStr) -> Option<F> {
    // SAFETY: a function pointer and an address have the same size; the caller vouches
    // for the signature.
    address(name).map(|address| unsafe { std::mem::transmute_copy(&address) })
}

fn load() -> Functions {
    // SAFETY: each alias above is the C signature of the function named here, as used by
    // other window tools and checked on macOS 27.2.
    unsafe {
        Functions {
            element_for_token: function(c"_AXUIElementCreateWithRemoteToken"),
            window_of_element: function(c"_AXUIElementGetWindow"),
            connection: function(c"CGSMainConnectionID"),
            display_spaces: function(c"CGSCopyManagedDisplaySpaces"),
            spaces_for_windows: function(c"CGSCopySpacesForWindows"),
            display_uuid: function(c"CGDisplayCreateUUIDFromDisplayID"),
        }
    }
}

pub fn functions() -> &'static Functions {
    static FUNCTIONS: OnceLock<Functions> = OnceLock::new();
    FUNCTIONS.get_or_init(load)
}
