//! The menu bar menu: what it lists, and what a click does.

use crate::flows::{self, Session};
use crate::ports::Host;
use crate::store::ProfileStore;

#[derive(Debug, Clone, PartialEq)]
pub enum MenuEntry {
    Save,
    Separator,
    Profile(String),
    NoProfiles,
    Dashboard,
    Quit,
}

impl MenuEntry {
    pub fn title(&self) -> String {
        match self {
            MenuEntry::Save => "Save Window Positions…".into(),
            MenuEntry::Separator => String::new(),
            MenuEntry::Profile(name) => name.clone(),
            MenuEntry::NoProfiles => "No Saved Profiles".into(),
            MenuEntry::Dashboard => "Open Dashboard…".into(),
            MenuEntry::Quit => "Quit Layoutsmart".into(),
        }
    }

    pub fn is_separator(&self) -> bool {
        *self == MenuEntry::Separator
    }

    pub fn is_enabled(&self) -> bool {
        !matches!(self, MenuEntry::Separator | MenuEntry::NoProfiles)
    }
}

pub fn entries(profiles: &[String]) -> Vec<MenuEntry> {
    let mut entries = vec![MenuEntry::Save, MenuEntry::Separator];
    if profiles.is_empty() {
        entries.push(MenuEntry::NoProfiles);
    }
    entries.extend(profiles.iter().cloned().map(MenuEntry::Profile));
    entries.extend([
        MenuEntry::Separator,
        MenuEntry::Dashboard,
        MenuEntry::Separator,
        MenuEntry::Quit,
    ]);
    entries
}

/// The menu as it was last shown. Items are identified by their position.
pub struct Menu {
    entries: Vec<MenuEntry>,
}

impl Menu {
    pub fn load(store: &ProfileStore) -> Menu {
        Menu {
            entries: entries(&store.names()),
        }
    }

    pub fn entries(&self) -> &[MenuEntry] {
        &self.entries
    }

    pub fn click(&self, index: usize, session: &Session, host: &dyn Host) {
        match self.entries.get(index) {
            Some(MenuEntry::Save) => {
                flows::save_flow(session);
            }
            Some(MenuEntry::Profile(name)) => {
                flows::restore_flow(session, name);
            }
            Some(MenuEntry::Dashboard) => host.open_dashboard(),
            Some(MenuEntry::Quit) => host.quit(),
            Some(MenuEntry::Separator | MenuEntry::NoProfiles) | None => {}
        }
    }
}

#[cfg(test)]
mod tests;
