//! The Layoutsmart menu bar app's window code (AppKit).
//!
//! Like `layoutsmart-macos`, this crate decides nothing: it shows what
//! `layoutsmart-core` tells it to and passes clicks back. Keep every function to at
//! most one branch; methods inside `define_class!` must be single calls.

pub mod ui;
