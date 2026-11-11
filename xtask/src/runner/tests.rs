use super::*;
use crate::cmd::Cmd;
use crate::testing::FakeRunner;

fn cmds() -> Vec<Cmd> {
    vec![Cmd::parse("one"), Cmd::parse("two x"), Cmd::parse("three")]
}

#[test]
fn runs_every_command_in_order_and_announces_each() {
    let runner = FakeRunner::default();
    assert!(run_all(&runner, &cmds()));
    assert_eq!(runner.log(), ["one", "two x", "three"]);
    assert_eq!(runner.said(), "▶ one\n▶ two x\n▶ three");
}

#[test]
fn stops_at_the_first_failure() {
    let runner = FakeRunner::failing_on("two");
    assert!(!run_all(&runner, &cmds()));
    assert_eq!(runner.log(), ["one", "two x"]);
    assert_eq!(runner.said(), "▶ one\n▶ two x\n✗ failed: two x");
}

#[test]
fn nothing_to_run_is_success() {
    assert!(run_all(&FakeRunner::default(), &[]));
}
