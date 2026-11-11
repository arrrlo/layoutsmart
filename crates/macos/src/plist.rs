//! Reading the dictionaries and lists macOS hands back, without trusting their shape:
//! anything missing or of the wrong kind is `None`.

use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2_foundation::{NSArray, NSDictionary, NSNumber, NSString};

pub type Dictionary = NSDictionary<NSString, AnyObject>;

fn as_dictionary(item: Retained<AnyObject>) -> Option<Retained<Dictionary>> {
    item.downcast::<NSDictionary>()
        .ok()
        // SAFETY: only the key type is asserted; a key of another kind just never matches.
        .map(|dictionary| unsafe { Retained::cast_unchecked(dictionary) })
}

/// The dictionaries in a list; anything else in it is skipped.
pub fn dictionaries(array: &NSArray) -> Vec<Retained<Dictionary>> {
    array.iter().filter_map(as_dictionary).collect()
}

fn value(dictionary: &Dictionary, key: &str) -> Option<Retained<AnyObject>> {
    dictionary.objectForKey(&NSString::from_str(key))
}

fn number(dictionary: &Dictionary, key: &str) -> Option<Retained<NSNumber>> {
    value(dictionary, key).and_then(|value| value.downcast::<NSNumber>().ok())
}

pub fn unsigned(dictionary: &Dictionary, key: &str) -> Option<u64> {
    number(dictionary, key).map(|number| number.unsignedLongLongValue())
}

pub fn signed(dictionary: &Dictionary, key: &str) -> Option<i64> {
    number(dictionary, key).map(|number| number.longLongValue())
}

pub fn float(dictionary: &Dictionary, key: &str) -> Option<f64> {
    number(dictionary, key).map(|number| number.doubleValue())
}

pub fn string(dictionary: &Dictionary, key: &str) -> Option<String> {
    value(dictionary, key)
        .and_then(|value| value.downcast::<NSString>().ok())
        .map(|string| string.to_string())
}

pub fn dictionary(dictionary: &Dictionary, key: &str) -> Option<Retained<Dictionary>> {
    value(dictionary, key).and_then(as_dictionary)
}

pub fn array(dictionary: &Dictionary, key: &str) -> Option<Retained<NSArray>> {
    value(dictionary, key).and_then(|value| value.downcast::<NSArray>().ok())
}
