//! Profiles on disk: one JSON file per profile. The file name is the profile name.

use crate::displays::same_display_set;
use crate::model::{DisplayInfo, Layout};
use std::path::{Path, PathBuf};
use std::{fmt, fs, io};

#[derive(Debug, Clone, PartialEq)]
pub enum StoreError {
    NotFound(String),
    Exists(String),
    Corrupt(String),
    Io(String),
}

impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StoreError::NotFound(name) => write!(f, "There is no saved profile named '{name}'."),
            StoreError::Exists(name) => write!(f, "A profile named '{name}' already exists."),
            StoreError::Corrupt(name) => {
                write!(f, "Profile '{name}' can't be read: its file is damaged.")
            }
            StoreError::Io(reason) => write!(f, "Couldn't write the profile: {reason}"),
        }
    }
}

fn io_error(error: io::Error) -> StoreError {
    StoreError::Io(error.to_string())
}

/// Where profiles live: `$LAYOUTSMART_DIR` if set, otherwise `~/.config/layoutsmart`.
pub fn default_dir(custom: Option<String>, home: Option<String>) -> PathBuf {
    match custom.filter(|dir| !dir.is_empty()) {
        Some(dir) => PathBuf::from(dir),
        None => PathBuf::from(home.unwrap_or_default()).join(".config/layoutsmart"),
    }
}

pub struct ProfileStore {
    dir: PathBuf,
}

impl ProfileStore {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        ProfileStore { dir: dir.into() }
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    pub fn path(&self, name: &str) -> PathBuf {
        self.dir.join(format!("{name}.json"))
    }

    pub fn exists(&self, name: &str) -> bool {
        self.path(name).is_file()
    }

    /// Profile names, sorted without regard to letter case.
    pub fn names(&self) -> Vec<String> {
        let mut names: Vec<String> = fs::read_dir(&self.dir)
            .into_iter()
            .flatten()
            .flatten()
            .filter_map(|entry| entry.file_name().into_string().ok())
            .filter_map(|file| file.strip_suffix(".json").map(String::from))
            .collect();
        names.sort_by_key(|name| name.to_lowercase());
        names
    }

    pub fn load(&self, name: &str) -> Result<Layout, StoreError> {
        let json =
            fs::read_to_string(self.path(name)).map_err(|_| StoreError::NotFound(name.into()))?;
        let mut layout = Layout::from_json(&json).map_err(|_| StoreError::Corrupt(name.into()))?;
        // People rename files in Finder; the file name wins over what is written inside.
        layout.profile = name.to_string();
        Ok(layout)
    }

    /// Writes `layout` under its profile name, replacing any existing file in one step.
    pub fn save(&self, layout: &Layout) -> Result<(), StoreError> {
        let temporary = self.dir.join(format!(".{}.json.tmp", layout.profile));
        fs::create_dir_all(&self.dir).map_err(io_error)?;
        fs::write(&temporary, layout.to_json()).map_err(io_error)?;
        fs::rename(&temporary, self.path(&layout.profile)).map_err(io_error)
    }

    /// True when `a` and `b` are different files (not just different spellings of one).
    fn are_distinct_files(&self, a: &str, b: &str) -> bool {
        fs::canonicalize(self.path(a)).ok() != fs::canonicalize(self.path(b)).ok()
    }

    pub fn rename(&self, old: &str, new: &str) -> Result<(), StoreError> {
        if !self.exists(old) {
            return Err(StoreError::NotFound(old.into()));
        }
        if self.exists(new) && self.are_distinct_files(old, new) {
            return Err(StoreError::Exists(new.into()));
        }
        fs::rename(self.path(old), self.path(new)).map_err(io_error)
    }

    pub fn delete(&self, name: &str) -> Result<(), StoreError> {
        fs::remove_file(self.path(name)).map_err(|_| StoreError::NotFound(name.into()))
    }

    /// Every readable profile, in `names()` order.
    pub fn layouts(&self) -> Vec<Layout> {
        self.names()
            .iter()
            .filter_map(|name| self.load(name).ok())
            .collect()
    }

    /// The most recently saved profile that was recorded with exactly these displays.
    pub fn profile_for(&self, displays: &[DisplayInfo]) -> Option<String> {
        self.layouts()
            .into_iter()
            .filter(|layout| same_display_set(&layout.displays, displays))
            .max_by(|a, b| a.saved_at.cmp(&b.saved_at))
            .map(|layout| layout.profile)
    }
}

#[cfg(test)]
mod tests;
