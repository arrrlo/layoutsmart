//! Works out which windows to move where when a profile is restored.

use crate::geometry::Rect;
use crate::model::WindowInfo;
use crate::windows::LiveWindow;

/// One window to move: `handle` identifies it to the macOS layer.
#[derive(Debug, Clone, PartialEq)]
pub struct Move {
    pub handle: usize,
    pub frame: Rect,
    pub app: String,
    pub title: String,
}

/// Removes and returns the first unclaimed saved entry that satisfies `wanted`.
fn claim<'a>(
    unclaimed: &mut Vec<&'a WindowInfo>,
    wanted: impl Fn(&WindowInfo) -> bool,
) -> Option<&'a WindowInfo> {
    let index = unclaimed.iter().position(|s| wanted(s))?;
    Some(unclaimed.remove(index))
}

/// Pairs open windows with saved ones, app by app: first by exact title, then — because
/// titles change (browser tabs, documents) — the leftovers in the order macOS lists them.
/// Windows already at their saved frame are left out.
pub fn plan_restore(saved: &[WindowInfo], live: &[LiveWindow]) -> Vec<Move> {
    let mut unclaimed: Vec<&WindowInfo> = saved.iter().collect();
    let mut pairs: Vec<(&LiveWindow, &WindowInfo)> = Vec::new();
    let mut untitled: Vec<&LiveWindow> = Vec::new();

    for window in live {
        let same_title =
            |s: &WindowInfo| s.bundle_id == window.bundle_id && s.title == window.title;
        match claim(&mut unclaimed, same_title) {
            Some(entry) => pairs.push((window, entry)),
            None => untitled.push(window),
        }
    }
    for window in untitled {
        let same_app = |s: &WindowInfo| s.bundle_id == window.bundle_id;
        pairs.extend(claim(&mut unclaimed, same_app).map(|entry| (window, entry)));
    }

    pairs
        .into_iter()
        .filter(|(window, entry)| !window.frame.integral_eq(&entry.frame()))
        .map(|(window, entry)| Move {
            handle: window.handle,
            frame: entry.frame(),
            app: window.app.clone(),
            title: window.title.clone(),
        })
        .collect()
}

#[cfg(test)]
mod tests;
