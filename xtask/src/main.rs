//! `cargo xtask <command>` — see `cargo xtask help`.

fn main() {
    // Always work from the workspace root, wherever cargo was invoked.
    let _ = std::env::set_current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/.."));
    let args: Vec<String> = std::env::args().skip(1).collect();
    let signing = xtask::bundle::Signing::from_env(|key| std::env::var(key).ok());
    std::process::exit(xtask::run_signed(&args, &xtask::sh::Shell, &signing));
}
