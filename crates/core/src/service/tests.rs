use super::*;
use crate::model::{SavedScreen, Showing, WindowInfo};
use crate::names::NameError;
use crate::store::{ProfileStore, StoreError};
use crate::testing::*;
use crate::windows::RawWindow;

fn with<'a>(sys: &'a FakeSystem, store: &'a ProfileStore, clock: &'a FixedClock) -> Backend<'a> {
    Backend { sys, store, clock }
}

const CLOCK: FixedClock = FixedClock(NOW);

#[test]
fn save_records_displays_and_managed_windows() {
    let (_dir, store) = temp_store();
    let mut sys = desk();
    sys.windows.push(RawWindow {
        minimized: Some(true),
        ..window("Notes", "hidden", 0.0)
    });
    let saved = save(&with(&sys, &store, &CLOCK), "  home ").unwrap();
    assert_eq!(
        saved,
        Saved {
            profile: "home".into(),
            windows: 3,
            displays: vec!["Built-in".into(), "DELL".into()],
            path: store.path("home"),
        }
    );
    let layout = store.load("home").unwrap();
    assert_eq!(layout.saved_at, NOW_ISO);
    let ids: Vec<&str> = layout.displays.iter().map(|d| d.id.as_str()).collect();
    assert_eq!(ids, ["1-2-10", "1-2-20"]);
    assert_eq!(layout.windows.len(), 3);
    assert_eq!(
        layout.windows[1],
        WindowInfo::new("Ghostty", "com.test.ghostty", "zsh", frame(2000.0))
    );
}

#[test]
fn save_rejects_invalid_names_before_touching_anything() {
    let (_dir, store) = temp_store();
    let sys = desk().untrusted();
    assert_eq!(
        save(&with(&sys, &store, &CLOCK), "a/b"),
        Err(Error::Name(NameError::InvalidChar('/')))
    );
    assert_eq!(sys.prompts.get(), 0);
    assert!(store.names().is_empty());
}

#[test]
fn save_needs_accessibility_permission() {
    let (_dir, store) = temp_store();
    let sys = desk().untrusted();
    assert_eq!(
        save(&with(&sys, &store, &CLOCK), "home"),
        Err(Error::NotTrusted)
    );
    assert_eq!(sys.prompts.get(), 1);
    assert!(store.names().is_empty());
}

#[test]
fn save_reports_storage_failures() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("file"), "x").unwrap();
    let store = ProfileStore::new(dir.path().join("file").join("sub"));
    let result = save(&with(&desk(), &store, &CLOCK), "home");
    assert!(matches!(result, Err(Error::Store(StoreError::Io(_)))));
}

#[test]
fn restore_moves_windows_back_to_their_saved_frames() {
    let (_dir, store) = temp_store();
    save(&with(&desk(), &store, &CLOCK), "home").unwrap();

    let mut shuffled = desk();
    shuffled.windows[0] = window("Safari", "Apple", 900.0);
    shuffled.windows[2] = window("Mail", "Inbox", 0.0);
    let restored = restore(&with(&shuffled, &store, &CLOCK), "home").unwrap();

    assert_eq!(restored.profile, "home");
    assert_eq!(shuffled.moves(), [(0, frame(100.0)), (2, frame(500.0))]);
    let apps: Vec<&str> = restored.moved.iter().map(|m| m.app.as_str()).collect();
    assert_eq!(apps, ["Safari", "Mail"]);
    assert!(restored.failed.is_empty());
}

#[test]
fn restore_reports_windows_that_refuse_to_move() {
    let (_dir, store) = temp_store();
    save(&with(&desk(), &store, &CLOCK), "home").unwrap();

    let mut shuffled = desk();
    shuffled.windows[0] = window("Safari", "Apple", 900.0);
    shuffled.windows[2] = window("Mail", "Inbox", 0.0);
    shuffled.stuck = vec![2];
    let restored = restore(&with(&shuffled, &store, &CLOCK), "home").unwrap();

    assert_eq!(shuffled.moves(), [(0, frame(100.0))]);
    assert_eq!(restored.moved.len(), 1);
    assert_eq!(restored.failed.len(), 1);
    assert_eq!(restored.failed[0].app, "Mail");
}

#[test]
fn restore_needs_permission_and_an_existing_profile() {
    let (_dir, store) = temp_store();
    let untrusted = desk().untrusted();
    assert_eq!(
        restore(&with(&untrusted, &store, &CLOCK), "home"),
        Err(Error::NotTrusted)
    );
    assert_eq!(untrusted.prompts.get(), 1);
    assert_eq!(
        restore(&with(&desk(), &store, &CLOCK), "home"),
        Err(Error::Store(StoreError::NotFound("home".into())))
    );
}

#[test]
fn restore_without_a_name_uses_the_profile_for_the_connected_displays() {
    let (_dir, store) = temp_store();
    save(&with(&desk(), &store, &CLOCK), "desk").unwrap();
    let laptop_only = FakeSystem::new(
        vec![display("Built-in", 10, 0.0)],
        vec![window("Safari", "Apple", 5.0)],
    );
    save(&with(&laptop_only, &store, &CLOCK), "travel").unwrap();

    let backend = with(&laptop_only, &store, &CLOCK);
    assert_eq!(matching_profile(&backend), Some("travel".into()));
    assert_eq!(
        restore_or_matching(&backend, None).unwrap().profile,
        "travel"
    );
    assert_eq!(
        restore_or_matching(&backend, Some("desk")).unwrap().profile,
        "desk"
    );
}

#[test]
fn restore_without_a_name_fails_when_no_profile_matches() {
    let (_dir, store) = temp_store();
    let sys = desk();
    let backend = with(&sys, &store, &CLOCK);
    assert_eq!(matching_profile(&backend), None);
    assert_eq!(
        restore_or_matching(&backend, None),
        Err(Error::NoMatchingProfile)
    );
}

#[test]
fn lists_displays_and_managed_windows() {
    let (_dir, store) = temp_store();
    let sys = desk();
    let backend = with(&sys, &store, &CLOCK);
    let names: Vec<String> = displays(&backend).into_iter().map(|d| d.name).collect();
    assert_eq!(names, ["Built-in", "DELL"]);
    let titles: Vec<String> = inventory(&backend)
        .unwrap()
        .open(false)
        .into_iter()
        .map(|w| w.title)
        .collect();
    assert_eq!(titles, ["Apple", "zsh", "Inbox"]);
}

/// The desk with desktops: DELL shows desktop 2 of two; Built-in shows a full-screen app.
/// Safari is in view, Mail is on DELL's hidden desktop 1, Ghostty is full screen on Built-in,
/// and a Notes window on the hidden desktop has never been handed out by its app.
fn desk_with_desktops() -> FakeSystem {
    let mut sys = desk();
    sys.screens = vec![
        screen(20, 2, &[(1, false), (2, false)]),
        screen(10, 5, &[(5, true)]),
    ];
    sys.windows[0].window_id = Some(100);
    sys.windows[0].spaces = vec![2];
    sys.windows[1].window_id = Some(101);
    sys.windows[1].spaces = vec![5];
    sys.windows[1].fullscreen = Some(true);
    sys.windows[2].window_id = Some(102);
    sys.windows[2].spaces = vec![1];
    sys.server = vec![
        server_window("Safari", 100, &[2], 100.0),
        server_window("Ghostty", 101, &[5], 2000.0),
        server_window("Mail", 102, &[1], 500.0),
        server_window("Notes", 103, &[1], 700.0),
    ];
    sys
}

#[test]
fn inventory_covers_every_desktop() {
    let (_dir, store) = temp_store();
    let sys = desk_with_desktops();
    let all = inventory(&with(&sys, &store, &CLOCK)).unwrap();
    let seen: Vec<(String, String, bool)> = all
        .windows
        .iter()
        .map(|w| {
            (
                w.app.clone(),
                w.place.as_ref().unwrap().label(),
                w.fullscreen,
            )
        })
        .collect();
    assert_eq!(
        seen,
        [
            ("Safari".to_string(), "DELL · desktop 2".to_string(), false),
            (
                "Ghostty".to_string(),
                "Built-in · full screen".to_string(),
                true
            ),
            (
                "Mail".to_string(),
                "DELL · desktop 1 (hidden)".to_string(),
                false
            ),
        ]
    );
    assert_eq!(all.unreached.len(), 1);
    assert_eq!(all.unreached[0].app, "Notes");
    assert_eq!(all.unreached[0].window_id, 103);
    assert!(all.desktops.is_known());
    assert_eq!(desktop_map(&with(&sys, &store, &CLOCK)), all.desktops);
}

#[test]
fn apps_with_missing_windows_are_asked_again_by_number() {
    let (_dir, store) = temp_store();
    let mut sys = desk_with_desktops();
    // Notes (pid 1 in these fakes) hands out its hidden window when addressed by number.
    let mut notes = window("Notes", "Shopping", 700.0);
    notes.window_id = Some(103);
    notes.spaces = vec![1];
    sys.numbered = vec![(1, notes), (55, window("Other", "not asked for", 0.0))];
    let all = inventory(&with(&sys, &store, &CLOCK)).unwrap();
    // Asked for exactly the missing window: Notes' window 103.
    assert_eq!(*sys.scans.borrow(), [vec![(1, 103)]]);
    assert!(all.unreached.is_empty());
    let last = all.windows.last().unwrap();
    assert_eq!(
        (last.app.as_str(), last.title.as_str(), last.handle),
        ("Notes", "Shopping", 3)
    );
    assert_eq!(
        last.place.as_ref().unwrap().label(),
        "DELL · desktop 1 (hidden)"
    );
    assert_eq!(all.windows.len(), 4);
}

#[test]
fn no_app_is_scanned_when_every_window_is_already_known() {
    let (_dir, store) = temp_store();
    let mut sys = desk_with_desktops();
    sys.server.pop();
    let all = inventory(&with(&sys, &store, &CLOCK)).unwrap();
    assert!(sys.scans.borrow().is_empty());
    assert!(all.unreached.is_empty());
    assert_eq!(all.windows.len(), 3);
}

#[test]
fn every_missing_window_is_asked_for_in_one_request() {
    let (_dir, store) = temp_store();
    let mut sys = desk_with_desktops();
    sys.server.push(server_window("Notes", 104, &[1], 800.0));
    let mut other = server_window("Preview", 200, &[1], 900.0);
    other.pid = 2;
    sys.server.push(other);
    let all = inventory(&with(&sys, &store, &CLOCK)).unwrap();
    assert_eq!(*sys.scans.borrow(), [vec![(1, 103), (1, 104), (2, 200)]]);
    assert_eq!(all.unreached.len(), 3);
}

#[test]
fn inventory_without_desktop_information_still_lists_windows() {
    let (_dir, store) = temp_store();
    let mut sys = desk();
    sys.server = vec![server_window("Notes", 103, &[1], 700.0)];
    let all = inventory(&with(&sys, &store, &CLOCK)).unwrap();
    assert_eq!(all.windows.len(), 3);
    assert!(all.windows.iter().all(|w| w.place.is_none()));
    assert!(all.unreached.is_empty());
    assert!(!all.desktops.is_known());
}

#[test]
fn inventory_needs_permission() {
    let (_dir, store) = temp_store();
    let sys = desk_with_desktops().untrusted();
    assert_eq!(
        inventory(&with(&sys, &store, &CLOCK)),
        Err(Error::NotTrusted)
    );
}

#[test]
fn the_windows_a_profile_is_about_are_all_of_them_or_the_plain_ones_in_view() {
    let (_dir, store) = temp_store();
    let sys = desk_with_desktops();
    let all = inventory(&with(&sys, &store, &CLOCK)).unwrap();
    let apps = |whole: bool| -> Vec<(String, bool)> {
        all.open(whole)
            .into_iter()
            .map(|window| (window.app, window.handle.is_some()))
            .collect()
    };
    assert_eq!(apps(false), [("Safari".to_string(), true)]);
    // Full-screen windows come last; the window no app handed out has no handle.
    assert_eq!(
        apps(true),
        [
            ("Safari".to_string(), true),
            ("Mail".to_string(), true),
            ("Notes".to_string(), false),
            ("Ghostty".to_string(), true),
        ]
    );
}

#[test]
fn save_records_where_every_window_is_and_what_each_screen_shows() {
    let (_dir, store) = temp_store();
    let sys = desk_with_desktops();
    let saved = save(&with(&sys, &store, &CLOCK), "home").unwrap();
    assert_eq!(saved.windows, 4);
    let layout = store.load("home").unwrap();
    assert_eq!(
        layout.windows,
        [
            WindowInfo::new("Safari", "com.test.safari", "Apple", frame(100.0))
                .on_desktop("1-2-20", 2),
            WindowInfo::new("Mail", "com.test.mail", "Inbox", frame(500.0)).on_desktop("1-2-20", 1),
            WindowInfo::new("Notes", "com.test.notes", "", frame(700.0)).on_desktop("1-2-20", 1),
            WindowInfo::new("Ghostty", "com.test.ghostty", "zsh", frame(2000.0))
                .in_full_screen("1-2-10"),
        ]
    );
    assert_eq!(
        layout.desktops,
        [
            SavedScreen {
                display: "1-2-20".into(),
                desktops: 2,
                showing: Showing::Desktop(2),
            },
            SavedScreen {
                display: "1-2-10".into(),
                desktops: 0,
                showing: Showing::FullScreen {
                    bundle_id: "com.test.ghostty".into(),
                    title: "zsh".into(),
                },
            },
        ]
    );
}

#[test]
fn what_a_screen_shows_is_unknown_when_macos_does_not_say_or_no_window_is_there() {
    let (_dir, store) = temp_store();
    let mut sys = desk_with_desktops();
    // DELL: the desktop in view is not in its row. Built-in: a full-screen desktop that
    // holds no window anyone knows.
    sys.screens = vec![
        screen(20, 9, &[(1, false), (2, false)]),
        screen(10, 6, &[(5, true), (6, true)]),
    ];
    save(&with(&sys, &store, &CLOCK), "home").unwrap();
    let showing: Vec<Showing> = store
        .load("home")
        .unwrap()
        .desktops
        .into_iter()
        .map(|screen| screen.showing)
        .collect();
    assert_eq!(showing, [Showing::Unknown, Showing::Unknown]);
}

#[test]
fn a_full_screen_desktop_is_told_apart_by_screen_and_by_position() {
    let (_dir, store) = temp_store();
    let mut sys = desk_with_desktops();
    // Built-in shows its second full-screen desktop, where Mail is; DELL has a
    // full-screen desktop at the same position in its row, holding Safari.
    sys.screens = vec![
        screen(20, 1, &[(1, false), (2, true)]),
        screen(10, 6, &[(5, true), (6, true)]),
    ];
    sys.windows[0].fullscreen = Some(true);
    sys.windows[2].spaces = vec![6];
    sys.windows[2].fullscreen = Some(true);
    sys.server.truncate(3);
    sys.server[2].spaces = vec![6];
    save(&with(&sys, &store, &CLOCK), "home").unwrap();
    assert_eq!(
        store.load("home").unwrap().desktops[1].showing,
        Showing::FullScreen {
            bundle_id: "com.test.mail".into(),
            title: "Inbox".into(),
        }
    );
}

#[test]
fn listing_windows_needs_permission_but_displays_do_not() {
    let (_dir, store) = temp_store();
    let sys = desk().untrusted();
    let backend = with(&sys, &store, &CLOCK);
    assert_eq!(inventory(&backend), Err(Error::NotTrusted));
    assert_eq!(displays(&backend).len(), 2);
}

#[test]
fn rename_validates_the_new_name() {
    let (_dir, store) = temp_store();
    let sys = desk();
    let backend = with(&sys, &store, &CLOCK);
    save(&backend, "home").unwrap();
    assert_eq!(
        rename(&backend, "home", "save"),
        Err(Error::Name(NameError::Reserved("save".into())))
    );
    assert_eq!(rename(&backend, "home", " house "), Ok("house".into()));
    assert_eq!(store.names(), ["house"]);
    assert_eq!(
        rename(&backend, "home", "flat"),
        Err(Error::Store(StoreError::NotFound("home".into())))
    );
}

#[test]
fn delete_removes_a_profile() {
    let (_dir, store) = temp_store();
    let sys = desk();
    let backend = with(&sys, &store, &CLOCK);
    save(&backend, "home").unwrap();
    assert_eq!(delete(&backend, "home"), Ok(()));
    assert!(store.names().is_empty());
    assert_eq!(
        delete(&backend, "home"),
        Err(Error::Store(StoreError::NotFound("home".into())))
    );
}

#[test]
fn error_messages() {
    assert_eq!(
        Error::NotTrusted.to_string(),
        "Layoutsmart needs Accessibility permission to see and move windows. \
         Turn it on in System Settings → Privacy & Security → Accessibility."
    );
    assert_eq!(
        Error::NoMatchingProfile.to_string(),
        "No saved profile matches the displays that are connected now."
    );
    assert_eq!(
        Error::Name(NameError::Empty).to_string(),
        NameError::Empty.to_string()
    );
    assert_eq!(
        Error::Store(StoreError::NotFound("x".into())).to_string(),
        StoreError::NotFound("x".into()).to_string()
    );
}

#[test]
fn types_are_cloneable_and_printable() {
    let e = Error::NotTrusted;
    assert_eq!(e.clone(), e);
    assert_eq!(format!("{e:?}"), "NotTrusted");
    let (_dir, store) = temp_store();
    let sys = desk();
    let backend = with(&sys, &store, &CLOCK);
    let saved = save(&backend, "home").unwrap();
    assert_eq!(saved.clone(), saved);
    assert!(format!("{saved:?}").starts_with("Saved"));
    let restored = restore(&backend, "home").unwrap();
    assert_eq!(restored.clone(), restored);
    assert!(format!("{restored:?}").starts_with("Restored"));
    let all = inventory(&backend).unwrap();
    assert_eq!(all.clone(), all);
    assert!(format!("{all:?}").starts_with("Inventory"));
}
