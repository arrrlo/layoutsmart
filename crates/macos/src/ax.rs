//! The Accessibility API: the only way to see and move other apps' windows.

use crate::apps::App;
use crate::private::functions;
use crate::spaces;
use layoutsmart_core::geometry::Rect;
use layoutsmart_core::windows::RawWindow;
use objc2_application_services::{
    AXCopyMultipleAttributeOptions, AXError, AXIsProcessTrustedWithOptions, AXUIElement, AXValue,
    AXValueType, kAXTrustedCheckOptionPrompt,
};
use objc2_core_foundation::{
    CFArray, CFBoolean, CFData, CFDictionary, CFRetained, CFString, CFType, CGPoint, CGSize, Type,
};
use std::cell::RefCell;
use std::collections::HashSet;
use std::ffi::c_void;
use std::ptr::{self, NonNull};

/// Seconds to wait for an app to answer before moving on.
const TIMEOUT: f32 = 1.0;

/// What `read_window` asks for, in this order.
const ATTRIBUTES: [&str; 6] = [
    "AXSubrole",
    "AXMinimized",
    "AXFullScreen",
    "AXTitle",
    "AXPosition",
    "AXSize",
];

pub fn is_trusted(prompt: bool) -> bool {
    // SAFETY: the key is a constant CFString exported by ApplicationServices.
    let key: &CFString = unsafe { kAXTrustedCheckOptionPrompt };
    let options = CFDictionary::from_slices(&[key], &[CFBoolean::new(prompt)]);
    // SAFETY: the dictionary maps the documented key to a CFBoolean, as the API requires.
    unsafe { AXIsProcessTrustedWithOptions(Some(options.as_opaque())) }
}

/// Stops one unresponsive app from stalling everything.
pub fn limit_waiting() {
    // SAFETY: plain calls; the system-wide element sets the timeout for this whole process.
    unsafe { AXUIElement::new_system_wide().set_messaging_timeout(TIMEOUT) };
}

/// Takes ownership of an object returned through an out-pointer, if the call worked.
///
/// SAFETY: `pointer` must be null or a +1 reference to a `T`.
unsafe fn owned<T: Type>(error: AXError, pointer: *const T) -> Option<CFRetained<T>> {
    NonNull::new(pointer.cast_mut())
        .filter(|_| error == AXError::Success)
        // SAFETY: guaranteed by the caller.
        .map(|pointer| unsafe { CFRetained::from_raw(pointer) })
}

fn attribute(element: &AXUIElement, name: &str) -> Option<CFRetained<CFType>> {
    let mut value: *const CFType = ptr::null();
    // SAFETY: `value` is a valid out-pointer; on success it receives a +1 reference.
    unsafe {
        let error =
            element.copy_attribute_value(&CFString::from_str(name), NonNull::from(&mut value));
        owned(error, value)
    }
}

type Values = Vec<Option<CFRetained<CFType>>>;

fn one_by_one(element: &AXUIElement) -> Values {
    ATTRIBUTES
        .iter()
        .map(|name| attribute(element, name))
        .collect()
}

/// All attributes in a single round trip to the app (six times fewer than `one_by_one`).
fn batched(element: &AXUIElement) -> Option<Values> {
    let names: Vec<CFRetained<CFString>> = ATTRIBUTES
        .iter()
        .map(|name| CFString::from_str(name))
        .collect();
    let names = CFArray::from_retained_objects(&names);
    let mut values: *const CFArray = ptr::null();
    // SAFETY: `values` is a valid out-pointer; on success it receives a +1 CFArray whose
    // items are CF objects (the attribute values, or an error placeholder per attribute).
    let values = unsafe {
        let error = element.copy_multiple_attribute_values(
            names.as_opaque(),
            AXCopyMultipleAttributeOptions(0),
            NonNull::from(&mut values),
        );
        CFRetained::cast_unchecked::<CFArray<CFType>>(owned(error, values)?)
    };
    Some(values.to_vec().into_iter().map(Some).collect())
}

fn item(values: &Values, index: usize) -> Option<&CFType> {
    values.get(index).and_then(|value| value.as_deref())
}

fn string(values: &Values, index: usize) -> Option<String> {
    item(values, index)
        .and_then(|value| value.downcast_ref::<CFString>())
        .map(|string| string.to_string())
}

fn boolean(values: &Values, index: usize) -> Option<bool> {
    item(values, index)
        .and_then(|value| value.downcast_ref::<CFBoolean>())
        .map(|boolean| boolean.as_bool())
}

fn point(values: &Values, index: usize) -> Option<(f64, f64)> {
    let value = item(values, index).and_then(|value| value.downcast_ref::<AXValue>())?;
    let mut point = CGPoint { x: 0.0, y: 0.0 };
    // SAFETY: `point` is a CGPoint, matching the requested type; false if the value isn't one.
    let ok = unsafe {
        value.value(
            AXValueType::CGPoint,
            NonNull::from(&mut point).cast::<c_void>(),
        )
    };
    ok.then_some((point.x, point.y))
}

fn size(values: &Values, index: usize) -> Option<(f64, f64)> {
    let value = item(values, index).and_then(|value| value.downcast_ref::<AXValue>())?;
    let mut size = CGSize {
        width: 0.0,
        height: 0.0,
    };
    // SAFETY: `size` is a CGSize, matching the requested type; false if the value isn't one.
    let ok = unsafe {
        value.value(
            AXValueType::CGSize,
            NonNull::from(&mut size).cast::<c_void>(),
        )
    };
    ok.then_some((size.width, size.height))
}

/// The window server's number for the window this element is (or is part of).
pub fn window_id(element: &AXUIElement) -> Option<u32> {
    let mut id = 0u32;
    functions()
        .window_of_element
        // SAFETY: `id` is a valid place for the answer.
        .map(|window_of_element| unsafe { window_of_element(element, &mut id) })
        .filter(|error| *error == AXError::Success)
        .map(|_| id)
}

/// Reports a window exactly as macOS describes it. The core decides what to make of it.
/// (The handle is filled in by the caller, who knows where the element is kept.)
pub fn read_window(element: &AXUIElement, app: &App) -> RawWindow {
    let values = batched(element).unwrap_or_else(|| one_by_one(element));
    let window_id = window_id(element);
    RawWindow {
        app: app.name.clone(),
        bundle_id: app.bundle_id.clone(),
        pid: app.pid,
        handle: 0,
        window_id,
        spaces: window_id.map(spaces::of_window).unwrap_or_default(),
        subrole: string(&values, 0),
        minimized: boolean(&values, 1),
        fullscreen: boolean(&values, 2),
        title: string(&values, 3),
        position: point(&values, 4),
        size: size(&values, 5),
    }
}

fn elements(array: CFRetained<CFArray>) -> Vec<CFRetained<AXUIElement>> {
    // SAFETY: the AXWindows attribute is documented to be an array of AXUIElements.
    unsafe { CFRetained::cast_unchecked::<CFArray<AXUIElement>>(array) }.to_vec()
}

/// The windows of one app, front to back.
pub fn windows_of(pid: i32) -> Vec<CFRetained<AXUIElement>> {
    // SAFETY: creating an element for a pid is always valid; a dead pid just answers nothing.
    let app = unsafe { AXUIElement::new_application(pid) };
    attribute(&app, "AXWindows")
        .and_then(|value| value.downcast::<CFArray>().ok())
        .map(elements)
        .unwrap_or_default()
}

/// The most element numbers tried per app. An app hands numbers out in order as its
/// windows and their parts are first listed; windows usually have numbers under 100.
const SCAN_LIMIT: u64 = 2000;

/// The token macOS uses to address element `number` of process `pid`.
fn token(pid: i32, number: u64) -> CFRetained<CFData> {
    let mut bytes = [0u8; 20];
    bytes[0..4].copy_from_slice(&pid.to_ne_bytes());
    bytes[8..12].copy_from_slice(&0x636f_636f_i32.to_ne_bytes());
    bytes[12..20].copy_from_slice(&number.to_ne_bytes());
    CFData::from_bytes(&bytes)
}

fn numbered_element(pid: i32, number: u64) -> Option<CFRetained<AXUIElement>> {
    functions()
        .element_for_token
        // SAFETY: the token has the layout the function expects; an unknown number just
        // gives an element that answers nothing.
        .and_then(|element_for_token| unsafe { element_for_token(&token(pid, number)) })
}

fn is_window(element: &AXUIElement) -> bool {
    attribute(element, "AXRole")
        .and_then(|role| role.downcast::<CFString>().ok())
        .is_some_and(|role| role.to_string() == "AXWindow")
}

/// Looks for the windows with these window-server numbers among the app's numbered
/// elements, and stops as soon as all are found. This reaches windows on desktops that
/// are not in view — but only ones the app has listed at some earlier time (see
/// CLAUDE.md, "Seeing windows").
pub fn numbered_windows(pid: i32, wanted: &[u32]) -> Vec<CFRetained<AXUIElement>> {
    // A window's parts (buttons, text fields) have numbers too and report the same
    // window number, so only the window element itself is taken.
    let missing: RefCell<HashSet<u32>> = RefCell::new(wanted.iter().copied().collect());
    (0..SCAN_LIMIT)
        .take_while(|_| !missing.borrow().is_empty())
        .filter_map(|number| numbered_element(pid, number))
        .filter(|element| window_id(element).is_some_and(|id| missing.borrow().contains(&id)))
        .filter(|element| is_window(element))
        .inspect(|element| {
            window_id(element).map(|id| missing.borrow_mut().remove(&id));
        })
        .collect()
}

fn set(element: &AXUIElement, name: &str, kind: AXValueType, value: NonNull<c_void>) -> bool {
    // SAFETY: callers pass a pointer to a value of the type named by `kind`.
    unsafe { AXValue::new(kind, value) }
        // SAFETY: a plain attribute write; the app may refuse, which is reported as false.
        .map(|value| unsafe { element.set_attribute_value(&CFString::from_str(name), &value) })
        .is_some_and(|error| error == AXError::Success)
}

fn set_position(element: &AXUIElement, frame: Rect) -> bool {
    let mut point = CGPoint {
        x: frame.x,
        y: frame.y,
    };
    let pointer = NonNull::from(&mut point).cast::<c_void>();
    set(element, "AXPosition", AXValueType::CGPoint, pointer)
}

fn set_size(element: &AXUIElement, frame: Rect) -> bool {
    let mut size = CGSize {
        width: frame.width,
        height: frame.height,
    };
    let pointer = NonNull::from(&mut size).cast::<c_void>();
    set(element, "AXSize", AXValueType::CGSize, pointer)
}

/// Moves first, so the size isn't limited by the display the window is leaving; then
/// resizes; then moves again, because some apps shift their origin when resized.
pub fn set_frame(element: &AXUIElement, frame: Rect) -> bool {
    let moved = set_position(element, frame);
    let sized = set_size(element, frame);
    set_position(element, frame);
    moved & sized
}
