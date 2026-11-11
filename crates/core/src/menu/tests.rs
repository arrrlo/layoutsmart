use super::MenuEntry::*;
use super::*;
use crate::flows::Session;
use crate::service::{self, Backend};
use crate::store::ProfileStore;
use crate::testing::Reply::Text;
use crate::testing::*;

fn session<'a>(sys: &'a FakeSystem, store: &'a ProfileStore, ui: &'a ScriptedUi) -> Session<'a> {
    Session {
        backend: Backend {
            sys,
            store,
            clock: &CLOCK,
        },
        ui,
    }
}

fn given_profile(store: &ProfileStore, name: &str) {
    let sys = desk();
    let backend = Backend {
        sys: &sys,
        store,
        clock: &CLOCK,
    };
    service::save(&backend, name).unwrap();
}

#[test]
fn lists_save_then_profiles_then_dashboard_then_quit() {
    let names = ["home".to_string(), "office".to_string()];
    assert_eq!(
        entries(&names),
        [
            Save,
            Separator,
            Profile("home".into()),
            Profile("office".into()),
            Separator,
            Dashboard,
            Separator,
            Quit,
        ]
    );
}

#[test]
fn says_so_when_there_are_no_profiles() {
    assert_eq!(
        entries(&[]),
        [
            Save, Separator, NoProfiles, Separator, Dashboard, Separator, Quit
        ]
    );
}

#[test]
fn titles() {
    assert_eq!(Save.title(), "Save Window Positions…");
    assert_eq!(Profile("no monitors".into()).title(), "no monitors");
    assert_eq!(NoProfiles.title(), "No Saved Profiles");
    assert_eq!(Dashboard.title(), "Open Dashboard…");
    assert_eq!(Quit.title(), "Quit Layoutsmart");
    assert_eq!(Separator.title(), "");
}

#[test]
fn only_real_actions_can_be_clicked() {
    assert!(Save.is_enabled());
    assert!(Profile("x".into()).is_enabled());
    assert!(Dashboard.is_enabled());
    assert!(Quit.is_enabled());
    assert!(!NoProfiles.is_enabled());
    assert!(!Separator.is_enabled());
    assert!(Separator.is_separator());
    assert!(!Save.is_separator());
    assert!(!NoProfiles.is_separator());
}

#[test]
fn loads_profiles_from_the_store_in_order() {
    let (_dir, store) = temp_store();
    given_profile(&store, "office");
    given_profile(&store, "Home");
    let menu = Menu::load(&store);
    assert_eq!(
        menu.entries()[2..4],
        [Profile("Home".into()), Profile("office".into())]
    );
    assert_eq!(menu.entries().len(), 8);
}

#[test]
fn clicking_save_runs_the_save_conversation() {
    let (_dir, store) = temp_store();
    let (sys, ui, host) = (
        desk(),
        ScriptedUi::new(vec![Text("home")]),
        FakeHost::default(),
    );
    Menu::load(&store).click(0, &session(&sys, &store, &ui), &host);
    assert_eq!(store.names(), ["home"]);
    assert!(host.log.borrow().is_empty());
}

#[test]
fn clicking_a_profile_restores_it() {
    let (_dir, store) = temp_store();
    given_profile(&store, "home");
    let mut sys = desk();
    sys.windows[0] = window("Safari", "Apple", 900.0);
    let (ui, host) = (ScriptedUi::silent(), FakeHost::default());
    Menu::load(&store).click(2, &session(&sys, &store, &ui), &host);
    assert_eq!(sys.moves(), [(0, frame(100.0))]);
    assert!(ui.log().is_empty());
}

#[test]
fn dashboard_and_quit_are_passed_to_the_app() {
    let (_dir, store) = temp_store();
    let (sys, ui, host) = (desk(), ScriptedUi::silent(), FakeHost::default());
    let menu = Menu::load(&store);
    assert_eq!(menu.entries()[4], Dashboard);
    menu.click(4, &session(&sys, &store, &ui), &host);
    menu.click(6, &session(&sys, &store, &ui), &host);
    assert_eq!(*host.log.borrow(), ["dashboard", "quit"]);
}

#[test]
fn clicks_on_nothing_do_nothing() {
    let (_dir, store) = temp_store();
    let (sys, ui, host) = (desk(), ScriptedUi::silent(), FakeHost::default());
    let menu = Menu::load(&store);
    for index in [1, 2, 3, 99] {
        menu.click(index, &session(&sys, &store, &ui), &host);
    }
    assert!(ui.log().is_empty());
    assert!(host.log.borrow().is_empty());
    assert!(sys.moves().is_empty());
}

#[test]
fn entry_is_cloneable_and_printable() {
    assert_eq!(Save.clone(), Save);
    assert_eq!(format!("{:?}", Profile("x".into())), "Profile(\"x\")");
}
