//! The operations behind both the CLI and the menu bar app.

use crate::desktops::DesktopMap;
use crate::displays::display_infos;
use crate::matcher::{Move, plan_restore};
use crate::model::{DisplayInfo, Layout, WindowInfo};
use crate::names::{self, NameError};
use crate::ports::{Clock, WindowSystem};
use crate::store::{ProfileStore, StoreError};
use crate::timefmt::iso8601;
use crate::windows::{LiveWindow, Unreached, live_windows, unreached};
use std::fmt;
use std::path::PathBuf;

/// Everything an operation needs from outside the core.
pub struct Backend<'a> {
    pub sys: &'a dyn WindowSystem,
    pub store: &'a ProfileStore,
    pub clock: &'a dyn Clock,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Error {
    NotTrusted,
    NoMatchingProfile,
    Name(NameError),
    Store(StoreError),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::NotTrusted => write!(
                f,
                "Layoutsmart needs Accessibility permission to see and move windows. \
                 Turn it on in System Settings → Privacy & Security → Accessibility."
            ),
            Error::NoMatchingProfile => write!(
                f,
                "No saved profile matches the displays that are connected now."
            ),
            Error::Name(error) => write!(f, "{error}"),
            Error::Store(error) => write!(f, "{error}"),
        }
    }
}

impl From<NameError> for Error {
    fn from(error: NameError) -> Self {
        Error::Name(error)
    }
}

impl From<StoreError> for Error {
    fn from(error: StoreError) -> Self {
        Error::Store(error)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Saved {
    pub profile: String,
    pub windows: usize,
    pub displays: Vec<String>,
    pub path: PathBuf,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Restored {
    pub profile: String,
    pub moved: Vec<Move>,
    /// Windows that should have moved but refused.
    pub failed: Vec<Move>,
}

fn require_trust(backend: &Backend) -> Result<(), Error> {
    if backend.sys.is_trusted(true) {
        Ok(())
    } else {
        Err(Error::NotTrusted)
    }
}

pub fn displays(backend: &Backend) -> Vec<DisplayInfo> {
    display_infos(&backend.sys.displays())
}

/// Which desktop of which screen each desktop id stands for.
pub fn desktop_map(backend: &Backend) -> DesktopMap {
    DesktopMap::new(&backend.sys.displays(), &backend.sys.screens())
}

/// Everything open right now, on every desktop.
#[derive(Debug, Clone, PartialEq)]
pub struct Inventory {
    /// Windows that can be read and moved.
    pub windows: Vec<LiveWindow>,
    /// Windows that exist but cannot be reached until their desktop has been in view.
    pub unreached: Vec<Unreached>,
    pub desktops: DesktopMap,
}

pub fn inventory(backend: &Backend) -> Result<Inventory, Error> {
    require_trust(backend)?;
    let desktops = desktop_map(backend);
    let server = backend.sys.server_windows();
    let mut raw = backend.sys.windows();
    // Windows the window server knows but their apps did not list get a second,
    // costlier look.
    let missing: Vec<(i32, u32)> = unreached(&raw, &server, &desktops)
        .iter()
        .map(|window| (window.pid, window.window_id))
        .collect();
    if !missing.is_empty() {
        raw.extend(backend.sys.numbered_windows(&missing));
    }
    Ok(Inventory {
        windows: live_windows(&raw, &desktops),
        unreached: unreached(&raw, &server, &desktops),
        desktops,
    })
}

/// The windows version 0.1 handled: ordinary ones on the desktops in view.
pub fn windows(backend: &Backend) -> Result<Vec<LiveWindow>, Error> {
    let all = inventory(backend)?.windows;
    Ok(all
        .into_iter()
        .filter(LiveWindow::is_plain_in_view)
        .collect())
}

/// The profile saved with the displays that are connected right now, if any.
pub fn matching_profile(backend: &Backend) -> Option<String> {
    backend.store.profile_for(&displays(backend))
}

pub fn save(backend: &Backend, name: &str) -> Result<Saved, Error> {
    let profile = names::validate(name)?;
    let layout = Layout {
        profile: profile.clone(),
        saved_at: iso8601(backend.clock.unix_seconds()),
        displays: displays(backend),
        windows: windows(backend)?
            .iter()
            .map(|w| WindowInfo::new(&w.app, &w.bundle_id, &w.title, w.frame))
            .collect(),
    };
    backend.store.save(&layout)?;
    Ok(Saved {
        path: backend.store.path(&profile),
        profile,
        windows: layout.windows.len(),
        displays: layout.displays.into_iter().map(|d| d.name).collect(),
    })
}

pub fn restore(backend: &Backend, name: &str) -> Result<Restored, Error> {
    let live = windows(backend)?;
    let layout = backend.store.load(name)?;
    let (moved, failed) = plan_restore(&layout.windows, &live)
        .into_iter()
        .partition(|m| backend.sys.set_frame(m.handle, m.frame));
    Ok(Restored {
        profile: layout.profile,
        moved,
        failed,
    })
}

/// Restores `name`, or — without a name — the profile for the connected displays.
pub fn restore_or_matching(backend: &Backend, name: Option<&str>) -> Result<Restored, Error> {
    match name {
        Some(name) => restore(backend, name),
        None => {
            let name = matching_profile(backend).ok_or(Error::NoMatchingProfile)?;
            restore(backend, &name)
        }
    }
}

/// Returns the (validated) new name.
pub fn rename(backend: &Backend, old: &str, new: &str) -> Result<String, Error> {
    let new = names::validate(new)?;
    backend.store.rename(old, &new)?;
    Ok(new)
}

pub fn delete(backend: &Backend, name: &str) -> Result<(), Error> {
    Ok(backend.store.delete(name)?)
}

#[cfg(test)]
mod tests;
