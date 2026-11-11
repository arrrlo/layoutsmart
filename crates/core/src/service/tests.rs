use super::*;
use crate::model::WindowInfo;
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
    let titles: Vec<String> = windows(&backend)
        .unwrap()
        .into_iter()
        .map(|w| w.title)
        .collect();
    assert_eq!(titles, ["Apple", "zsh", "Inbox"]);
}

#[test]
fn listing_windows_needs_permission_but_displays_do_not() {
    let (_dir, store) = temp_store();
    let sys = desk().untrusted();
    let backend = with(&sys, &store, &CLOCK);
    assert_eq!(windows(&backend), Err(Error::NotTrusted));
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
}
