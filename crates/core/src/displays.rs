//! Display identity, and whether two display sets are the same monitors.

use crate::geometry::Rect;
use crate::model::DisplayInfo;

/// What the macOS layer reports for one active display.
#[derive(Debug, Clone, PartialEq)]
pub struct RawDisplay {
    pub name: Option<String>,
    pub vendor: u32,
    pub model: u32,
    pub serial: u32,
    /// Global, top-left-origin bounds (the same space window positions use).
    pub bounds: Rect,
}

/// `vendor-model-serial`: stays the same when a monitor is unplugged and plugged back in.
pub fn display_identity(vendor: u32, model: u32, serial: u32) -> String {
    format!("{vendor}-{model}-{serial}")
}

pub fn display_infos(raw: &[RawDisplay]) -> Vec<DisplayInfo> {
    raw.iter()
        .enumerate()
        .map(|(i, d)| DisplayInfo {
            name: d
                .name
                .clone()
                .unwrap_or_else(|| format!("Display {}", i + 1)),
            id: display_identity(d.vendor, d.model, d.serial),
            x: d.bounds.x,
            y: d.bounds.y,
            width: d.bounds.width,
            height: d.bounds.height,
        })
        .collect()
}

fn sorted_ids(displays: &[DisplayInfo]) -> Vec<&str> {
    let mut ids: Vec<&str> = displays.iter().map(|d| d.id.as_str()).collect();
    ids.sort_unstable();
    ids
}

/// True when both lists hold the same monitors, in any order or arrangement.
pub fn same_display_set(a: &[DisplayInfo], b: &[DisplayInfo]) -> bool {
    sorted_ids(a) == sorted_ids(b)
}

#[cfg(test)]
mod tests;
