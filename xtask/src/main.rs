//! `cargo xtask <command>` — see `cargo xtask help`.

fn main() {
    // Always work from the workspace root, wherever cargo was invoked.
    let _ = std::env::set_current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/.."));
    let args: Vec<String> = std::env::args().skip(1).collect();
    std::process::exit(xtask::run(&args, &xtask::sh::Shell));
}
