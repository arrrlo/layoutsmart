use super::*;
use crate::geometry::Rect;
use crate::ports::WindowSystem;
use crate::restore::NO_STEPPING_NOTE;
use crate::restore::testing::Sim;
use crate::service::{self, Backend};
use crate::store::ProfileStore;
use crate::testing::Reply::{Cancel, No, Text, Yes};
use crate::testing::*;

fn session<'a>(
    sys: &'a dyn WindowSystem,
    store: &'a ProfileStore,
    ui: &'a ScriptedUi,
) -> Session<'a> {
    Session {
        backend: Backend {
            sys,
            store,
            clock: &CLOCK,
        },
        ui,
    }
}

/// Saves `name` from the standard desk, without any dialogs.
fn given_profile(store: &ProfileStore, name: &str) {
    let sys = desk();
    let backend = Backend {
        sys: &sys,
        store,
        clock: &CLOCK,
    };
    service::save(&backend, name).unwrap();
}

fn shuffled_desk() -> FakeSystem {
    let mut sys = desk();
    sys.windows[0] = window("Safari", "Apple", 900.0);
    sys.windows[2] = window("Mail", "Inbox", 0.0);
    sys
}

fn permission_dialog() -> String {
    format!(
        "confirm[Layoutsmart needs Accessibility permission|{PERMISSION_MESSAGE}|Open Settings|false]"
    )
}

// ---- save

#[test]
fn save_asks_for_a_name_and_saves() {
    let (_dir, store) = temp_store();
    let (sys, ui) = (desk(), ScriptedUi::new(vec![Text("  home ")]));
    assert_eq!(save_flow(&session(&sys, &store, &ui)), Some("home".into()));
    assert_eq!(store.names(), ["home"]);
    assert_eq!(
        ui.log(),
        [format!("ask[Save Window Positions|{SAVE_HINT}||Save]")]
    );
}

#[test]
fn save_cancelled_saves_nothing() {
    let (_dir, store) = temp_store();
    let (sys, ui) = (desk(), ScriptedUi::new(vec![Cancel]));
    assert_eq!(save_flow(&session(&sys, &store, &ui)), None);
    assert!(store.names().is_empty());
    assert_eq!(ui.log().len(), 1);
}

#[test]
fn save_explains_an_invalid_name_and_asks_again() {
    let (_dir, store) = temp_store();
    let (sys, ui) = (desk(), ScriptedUi::new(vec![Text("a/b"), Text("ok")]));
    assert_eq!(save_flow(&session(&sys, &store, &ui)), Some("ok".into()));
    assert_eq!(store.names(), ["ok"]);
    assert_eq!(
        ui.log()[1],
        format!("ask[Save Window Positions|Profile name can't contain '/'.\n{SAVE_HINT}|a/b|Save]")
    );
}

#[test]
fn save_asks_before_replacing_a_profile() {
    let (_dir, store) = temp_store();
    given_profile(&store, "home");
    let laptop = FakeSystem::new(vec![display("Built-in", 10, 0.0)], vec![]);
    let ui = ScriptedUi::new(vec![Text("home"), Yes]);
    assert_eq!(
        save_flow(&session(&laptop, &store, &ui)),
        Some("home".into())
    );
    assert_eq!(
        ui.log()[1],
        format!("confirm[Replace “home”?|{REPLACE_MESSAGE}|Replace|true]")
    );
    assert_eq!(store.load("home").unwrap().displays.len(), 1);
}

#[test]
fn declining_to_replace_asks_again_and_keeps_the_typed_name() {
    let (_dir, store) = temp_store();
    given_profile(&store, "home");
    let laptop = FakeSystem::new(vec![display("Built-in", 10, 0.0)], vec![]);
    let ui = ScriptedUi::new(vec![Text("home"), No, Text("home 2")]);
    assert_eq!(
        save_flow(&session(&laptop, &store, &ui)),
        Some("home 2".into())
    );
    assert_eq!(
        ui.log()[2],
        format!("ask[Save Window Positions|{SAVE_HINT}|home|Save]")
    );
    assert_eq!(store.names(), ["home", "home 2"]);
    assert_eq!(store.load("home").unwrap().displays.len(), 2);
}

#[test]
fn save_without_permission_offers_to_open_settings() {
    let (_dir, store) = temp_store();
    let (sys, ui) = (desk().untrusted(), ScriptedUi::new(vec![Yes, Text("home")]));
    assert_eq!(save_flow(&session(&sys, &store, &ui)), None);
    assert_eq!(ui.log(), [permission_dialog(), "open-settings".to_string()]);
    assert_eq!(sys.prompts.get(), 0);
    assert!(store.names().is_empty());
}

#[test]
fn declining_the_permission_dialog_does_not_open_settings() {
    let (_dir, store) = temp_store();
    let (sys, ui) = (desk().untrusted(), ScriptedUi::new(vec![No]));
    assert_eq!(save_flow(&session(&sys, &store, &ui)), None);
    assert_eq!(ui.log(), [permission_dialog()]);
}

#[test]
fn a_failed_save_is_reported() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("file"), "x").unwrap();
    let store = ProfileStore::new(dir.path().join("file").join("sub"));
    let (sys, ui) = (desk(), ScriptedUi::new(vec![Text("home")]));
    assert_eq!(save_flow(&session(&sys, &store, &ui)), None);
    let log = ui.log();
    assert_eq!(log.len(), 2);
    assert!(
        log[1].starts_with("alert[Couldn’t save the window positions|Couldn't write the profile: "),
        "{}",
        log[1]
    );
}

// ---- restore

#[test]
fn restore_moves_the_windows_without_any_dialog() {
    let (_dir, store) = temp_store();
    given_profile(&store, "home");
    let (sys, ui) = (shuffled_desk(), ScriptedUi::silent());
    assert!(restore_flow(&session(&sys, &store, &ui), "home"));
    assert_eq!(sys.moves(), [(0, frame(100.0)), (2, frame(500.0))]);
    assert!(ui.log().is_empty());
}

#[test]
fn restore_lists_windows_that_refused_to_move() {
    let (_dir, store) = temp_store();
    given_profile(&store, "home");
    let mut sys = shuffled_desk();
    sys.windows[0] = window("Safari", "", 900.0);
    sys.stuck = vec![0, 2];
    let ui = ScriptedUi::silent();
    // In the order of the profile; Safari's title is empty now.
    assert!(restore_flow(&session(&sys, &store, &ui), "home"));
    assert_eq!(
        ui.log(),
        ["alert[2 windows couldn’t be put back|Safari\nMail — Inbox]"]
    );
}

/// One screen with two desktops, the first in view, and a window on the second.
fn sim_with_a_window_on_a_hidden_desktop() -> (Sim, u32) {
    let sim = Sim::new().screen("Main", 10, Rect::new(0.0, 0.0, 2000.0, 1200.0), 2, 1);
    let id = sim.window(
        "Notes",
        "list",
        "Main",
        2,
        Rect::new(10.0, 20.0, 300.0, 200.0),
    );
    (sim, id)
}

#[test]
fn restore_says_when_a_window_is_left_on_the_wrong_desktop_and_why() {
    let (_dir, store) = temp_store();
    let (sim, id) = sim_with_a_window_on_a_hidden_desktop();
    let ui = ScriptedUi::silent();
    let session = session(&sim, &store, &ui);
    service::save(&session.backend, "home").unwrap();
    sim.user_moves(id, "Main", 1, Rect::new(10.0, 20.0, 300.0, 200.0));
    sim.shortcuts_on.set(false);

    assert!(restore_flow(&session, "home"));
    assert_eq!(
        ui.log(),
        [format!(
            "alert[1 window couldn’t be put back|Notes — list\n\n{NO_STEPPING_NOTE}]"
        )]
    );
}

#[test]
fn restore_says_when_only_the_desktops_in_view_could_not_be_put_back() {
    let (_dir, store) = temp_store();
    let (sim, _) = sim_with_a_window_on_a_hidden_desktop();
    let ui = ScriptedUi::silent();
    let session = session(&sim, &store, &ui);
    service::save(&session.backend, "home").unwrap();
    sim.user_shows("Main", 2);
    sim.shortcuts_on.set(false);

    assert!(restore_flow(&session, "home"));
    assert_eq!(
        ui.log(),
        [format!(
            "alert[“home” was restored, but not completely|{NO_STEPPING_NOTE}]"
        )]
    );
}

#[test]
fn restore_reports_a_single_stuck_window_in_the_singular() {
    let (_dir, store) = temp_store();
    given_profile(&store, "home");
    let mut sys = shuffled_desk();
    sys.stuck = vec![2];
    let ui = ScriptedUi::silent();
    assert!(restore_flow(&session(&sys, &store, &ui), "home"));
    assert_eq!(
        ui.log(),
        ["alert[1 window couldn’t be put back|Mail — Inbox]"]
    );
    assert_eq!(sys.moves(), [(0, frame(100.0))]);
}

#[test]
fn restoring_a_missing_profile_is_reported() {
    let (_dir, store) = temp_store();
    let (sys, ui) = (desk(), ScriptedUi::silent());
    assert!(!restore_flow(&session(&sys, &store, &ui), "nope"));
    assert_eq!(
        ui.log(),
        ["alert[Couldn’t restore “nope”|There is no saved profile named 'nope'.]"]
    );
}

#[test]
fn restore_without_permission_moves_nothing() {
    let (_dir, store) = temp_store();
    given_profile(&store, "home");
    let (sys, ui) = (shuffled_desk().untrusted(), ScriptedUi::new(vec![No]));
    assert!(!restore_flow(&session(&sys, &store, &ui), "home"));
    assert_eq!(ui.log(), [permission_dialog()]);
    assert!(sys.moves().is_empty());
}

// ---- rename

#[test]
fn rename_asks_with_the_current_name_filled_in() {
    let (_dir, store) = temp_store();
    given_profile(&store, "home");
    let (sys, ui) = (desk(), ScriptedUi::new(vec![Text(" house ")]));
    assert_eq!(
        rename_flow(&session(&sys, &store, &ui), "home"),
        Some("house".into())
    );
    assert_eq!(store.names(), ["house"]);
    assert_eq!(
        ui.log(),
        [format!("ask[Rename “home”|{RENAME_HINT}|home|Rename]")]
    );
}

#[test]
fn rename_cancelled_changes_nothing() {
    let (_dir, store) = temp_store();
    given_profile(&store, "home");
    let (sys, ui) = (desk(), ScriptedUi::new(vec![Cancel]));
    assert_eq!(rename_flow(&session(&sys, &store, &ui), "home"), None);
    assert_eq!(store.names(), ["home"]);
}

#[test]
fn rename_explains_an_invalid_name_and_asks_again() {
    let (_dir, store) = temp_store();
    given_profile(&store, "home");
    let (sys, ui) = (desk(), ScriptedUi::new(vec![Text(""), Text("house")]));
    assert_eq!(
        rename_flow(&session(&sys, &store, &ui), "home"),
        Some("house".into())
    );
    assert_eq!(
        ui.log()[1],
        format!("ask[Rename “home”|Profile name can't be empty.\n{RENAME_HINT}||Rename]")
    );
}

#[test]
fn rename_onto_an_existing_profile_is_reported() {
    let (_dir, store) = temp_store();
    given_profile(&store, "home");
    given_profile(&store, "office");
    let (sys, ui) = (desk(), ScriptedUi::new(vec![Text("office")]));
    assert_eq!(rename_flow(&session(&sys, &store, &ui), "home"), None);
    assert_eq!(
        ui.log()[1],
        "alert[Couldn’t rename “home”|A profile named 'office' already exists.]"
    );
    assert_eq!(store.names(), ["home", "office"]);
}

// ---- delete

#[test]
fn delete_asks_first() {
    let (_dir, store) = temp_store();
    given_profile(&store, "home");
    let (sys, ui) = (desk(), ScriptedUi::new(vec![Yes]));
    assert!(delete_flow(&session(&sys, &store, &ui), "home"));
    assert!(store.names().is_empty());
    assert_eq!(
        ui.log(),
        [format!(
            "confirm[Delete “home”?|{DELETE_MESSAGE}|Delete|true]"
        )]
    );
}

#[test]
fn delete_declined_keeps_the_profile() {
    let (_dir, store) = temp_store();
    given_profile(&store, "home");
    let (sys, ui) = (desk(), ScriptedUi::new(vec![No]));
    assert!(!delete_flow(&session(&sys, &store, &ui), "home"));
    assert_eq!(store.names(), ["home"]);
}

#[test]
fn a_failed_delete_is_reported() {
    let (_dir, store) = temp_store();
    let (sys, ui) = (desk(), ScriptedUi::new(vec![Yes]));
    assert!(!delete_flow(&session(&sys, &store, &ui), "ghost"));
    assert_eq!(
        ui.log()[1],
        "alert[Couldn’t delete “ghost”|There is no saved profile named 'ghost'.]"
    );
}
