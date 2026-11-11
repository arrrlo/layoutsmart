use super::*;
use crate::flows::Session;
use crate::service::Backend;
use crate::store::ProfileStore;
use crate::testing::*;

const INSTALLED: &str = "/Applications/Layoutsmart.app/Contents/MacOS/Layoutsmart";
const DEV_BUILD: &str =
    "/Users/me/code/layoutsmart/dist/Layoutsmart.app/Contents/MacOS/Layoutsmart";

fn launch(sys: &FakeSystem, store: &ProfileStore, login: &FakeLogin, exe: &str) {
    let ui = ScriptedUi::silent();
    let session = Session {
        backend: Backend {
            sys,
            store,
            clock: &CLOCK,
        },
        ui: &ui,
    };
    on_launch(&session, login, exe);
    assert!(ui.log().is_empty());
}

#[test]
fn registers_to_start_at_login_exactly_once() {
    let (_dir, store) = temp_store();
    let login = FakeLogin::new(true);
    launch(&desk(), &store, &login, INSTALLED);
    assert_eq!(login.calls.get(), 1);
    assert!(store.dir().join(LOGIN_MARKER).is_file());
    // Later launches leave the user's choice alone, even if they switched it off.
    launch(&desk(), &store, &login, INSTALLED);
    assert_eq!(login.calls.get(), 1);
    assert!(store.names().is_empty());
}

#[test]
fn a_refused_registration_is_tried_again_next_time() {
    let (_dir, store) = temp_store();
    let login = FakeLogin::new(false);
    launch(&desk(), &store, &login, INSTALLED);
    assert!(!store.dir().join(LOGIN_MARKER).exists());
    launch(&desk(), &store, &login, INSTALLED);
    assert_eq!(login.calls.get(), 2);
}

#[test]
fn a_development_build_never_registers() {
    let (_dir, store) = temp_store();
    let login = FakeLogin::new(true);
    launch(&desk(), &store, &login, DEV_BUILD);
    assert_eq!(login.calls.get(), 0);
    assert!(!store.dir().join(LOGIN_MARKER).exists());
}

#[test]
fn asks_macos_to_show_its_permission_prompt_when_needed() {
    let (_dir, store) = temp_store();
    let untrusted = desk().untrusted();
    launch(&untrusted, &store, &FakeLogin::new(true), INSTALLED);
    assert_eq!(untrusted.prompts.get(), 1);

    let trusted = desk();
    launch(&trusted, &store, &FakeLogin::new(true), INSTALLED);
    assert_eq!(trusted.prompts.get(), 0);
}

#[test]
fn installed_means_inside_an_applications_folder() {
    assert!(is_installed(INSTALLED));
    assert!(is_installed(
        "/Users/me/Applications/Layoutsmart.app/Contents/MacOS/Layoutsmart"
    ));
    assert!(!is_installed(DEV_BUILD));
    assert!(!is_installed("Applications/Layoutsmart.app"));
    assert!(!is_installed(""));
}

#[test]
fn dashboard_flag() {
    let args = |list: &[&str]| list.iter().map(|s| s.to_string()).collect::<Vec<_>>();
    assert!(wants_dashboard(&args(&["--dashboard"])));
    assert!(wants_dashboard(&args(&["-x", "--dashboard"])));
    assert!(!wants_dashboard(&args(&[])));
    assert!(!wants_dashboard(&args(&["dashboard"])));
}

#[test]
fn snapshot_flag_names_the_file_to_write() {
    let args = |list: &[&str]| list.iter().map(|s| s.to_string()).collect::<Vec<_>>();
    assert_eq!(
        snapshot_path(&args(&["app", "--snapshot", "out.png"])),
        Some("out.png")
    );
    assert_eq!(
        snapshot_path(&args(&["--snapshot", "a.png", "b"])),
        Some("a.png")
    );
    assert_eq!(snapshot_path(&args(&["app", "--snapshot"])), None);
    assert_eq!(snapshot_path(&args(&["app", "out.png"])), None);
    assert_eq!(snapshot_path(&args(&[])), None);
}
