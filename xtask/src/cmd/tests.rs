use super::*;

#[test]
fn parse_splits_on_whitespace() {
    let cmd = Cmd::parse("cargo  fmt --all\t--check");
    assert_eq!(cmd.program, "cargo");
    assert_eq!(cmd.args, ["fmt", "--all", "--check"]);
    assert!(cmd.env.is_empty());
}

#[test]
fn builders_append() {
    let cmd = Cmd::parse("codesign --force")
        .arg("My App.app")
        .args(&["-s".to_string(), "-".to_string()])
        .env("KEY", "value");
    assert_eq!(cmd.args, ["--force", "My App.app", "-s", "-"]);
    assert_eq!(cmd.env, [("KEY".to_string(), "value".to_string())]);
}

#[test]
fn displays_as_a_command_line() {
    assert_eq!(Cmd::parse("cargo test").to_string(), "cargo test");
    assert_eq!(Cmd::parse("ls").to_string(), "ls");
    assert_eq!(
        Cmd::parse("cargo build")
            .env("A", "1")
            .env("B", "2")
            .to_string(),
        "A=1 B=2 cargo build"
    );
}

#[test]
fn is_cloneable_and_printable() {
    let cmd = Cmd::parse("a b");
    assert_eq!(cmd.clone(), cmd);
    assert!(format!("{cmd:?}").starts_with("Cmd"));
}
