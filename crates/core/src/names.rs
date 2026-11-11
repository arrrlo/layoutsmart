//! Profile name validation.

use std::fmt;

pub const MAX_LEN: usize = 64;

/// Command words can't be profile names (`layoutsmart <name> save` would be ambiguous).
const RESERVED: &[&str] = &[
    "save", "restore", "list", "displays", "profiles", "rename", "delete", "help",
];

#[derive(Debug, Clone, PartialEq)]
pub enum NameError {
    Empty,
    TooLong,
    InvalidChar(char),
    InvalidStart(char),
    Reserved(String),
}

impl fmt::Display for NameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NameError::Empty => write!(f, "Profile name can't be empty."),
            NameError::TooLong => write!(f, "Profile name can be at most {MAX_LEN} characters."),
            NameError::InvalidChar(c) => write!(f, "Profile name can't contain {c:?}."),
            NameError::InvalidStart(c) => write!(f, "Profile name can't start with {c:?}."),
            NameError::Reserved(w) => {
                write!(f, "'{w}' is a command and can't be a profile name.")
            }
        }
    }
}

pub fn is_reserved(name: &str) -> bool {
    RESERVED.contains(&name)
}

fn invalid_char(name: &str) -> Option<char> {
    name.chars()
        .find(|c| matches!(c, '/' | ':') || c.is_control())
}

/// A leading `.` would hide the file; a leading `-` would look like a command-line flag.
fn invalid_start(name: &str) -> Option<char> {
    name.chars().next().filter(|c| matches!(c, '.' | '-'))
}

/// Returns the trimmed name if it can be used as a profile (and file) name.
pub fn validate(input: &str) -> Result<String, NameError> {
    let name = input.trim();
    if name.is_empty() {
        return Err(NameError::Empty);
    }
    if name.chars().count() > MAX_LEN {
        return Err(NameError::TooLong);
    }
    if let Some(c) = invalid_char(name) {
        return Err(NameError::InvalidChar(c));
    }
    if let Some(c) = invalid_start(name) {
        return Err(NameError::InvalidStart(c));
    }
    if is_reserved(name) {
        return Err(NameError::Reserved(name.into()));
    }
    Ok(name.into())
}

#[cfg(test)]
mod tests;
