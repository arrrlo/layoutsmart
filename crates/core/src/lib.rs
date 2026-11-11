//! Layoutsmart core: every decision lives here. No macOS dependencies, no unsafe code.
#![forbid(unsafe_code)]

pub mod cli;
pub mod displays;
pub mod geometry;
pub mod matcher;
pub mod model;
pub mod names;
pub mod ports;
pub mod service;
pub mod store;
pub mod timefmt;
pub mod windows;

#[cfg(test)]
mod testing;
