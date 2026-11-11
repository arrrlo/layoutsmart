//! Small wording helpers.

/// `1 window`, `3 windows`.
pub fn plural(count: usize, noun: &str) -> String {
    if count == 1 {
        format!("1 {noun}")
    } else {
        format!("{count} {noun}s")
    }
}

/// `“home”` — how names are shown in dialogs.
pub fn quoted(name: &str) -> String {
    format!("“{name}”")
}

#[cfg(test)]
mod tests;
