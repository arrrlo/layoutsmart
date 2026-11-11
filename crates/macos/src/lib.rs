//! Layoutsmart's macOS layer: it fetches raw facts and performs actions.
//!
//! Nothing here makes a decision — that all lives in `layoutsmart-core`, where it is
//! unit-tested. Keep every function in this crate to at most one branch.

mod apps;
mod ax;
mod input;
mod plist;
mod private;
mod screens;
mod server;
mod shortcuts;
mod spaces;
mod system;

pub use system::{MacSystem, SystemClock, default_store};
