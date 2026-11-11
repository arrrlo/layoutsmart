//! The Accessibility API: the only way to see and move other apps' windows.

use crate::apps::App;
use layoutsmart_core::geometry::Rect;
use layoutsmart_core::windows::RawWindow;
use objc2_application_services::{
    AXCopyMultipleAttributeOptions, AXError, AXIsProcessTrustedWithOptions, AXUIElement, AXValue,
    AXValueType, kAXTrustedCheckOptionPrompt,
};
use objc2_core_foundation::{
    CFArray, CFBoolean, CFDictionary, CFRetained, CFString, CFType, CGPoint, CGSize, Type,
};
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

/// Reports a window exactly as macOS describes it. The core decides what to make of it.
pub fn read_window(element: &AXUIElement, app: &App) -> RawWindow {
    let values = batched(element).unwrap_or_else(|| one_by_one(element));
    RawWindow {
        app: app.name.clone(),
        bundle_id: app.bundle_id.clone(),
        pid: app.pid,
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
