//! Layoutsmart core: every decision lives here. No macOS dependencies, no unsafe code.
#![forbid(unsafe_code)]

pub mod displays;
pub mod geometry;
pub mod matcher;
pub mod model;
pub mod names;
pub mod timefmt;
pub mod windows;
