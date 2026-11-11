//! The `layoutsmart` command. All behaviour is in `layoutsmart_core::cli`.

use layoutsmart_core::cli;
use layoutsmart_core::service::Backend;
use layoutsmart_macos::{MacSystem, SystemClock, default_store};
use std::io::{stderr, stdout};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (sys, store) = (MacSystem::new(), default_store());
    let backend = Backend {
        sys: &sys,
        store: &store,
        clock: &SystemClock,
    };
    std::process::exit(cli::run(&args, &backend, &mut stdout(), &mut stderr()));
}
