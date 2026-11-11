use super::*;
use crate::geometry::Rect;
use crate::model::{DisplayInfo, Layout, WindowInfo};
use crate::testing::temp_store;
use std::fs;
use std::path::PathBuf;

fn display(id: &str) -> DisplayInfo {
    DisplayInfo {
        name: format!("Display {id}"),
        id: id.into(),
        x: 0.0,
        y: 0.0,
        width: 1920.0,
        height: 1080.0,
    }
}

fn layout(name: &str, saved_at: &str, display_ids: &[&str]) -> Layout {
    Layout {
        profile: name.into(),
        saved_at: saved_at.into(),
        displays: display_ids.iter().map(|id| display(id)).collect(),
        windows: vec![WindowInfo::new(
            "App",
            "com.app",
            "win",
            Rect::new(1.0, 2.0, 3.0, 4.0),
        )],
    }
}

fn simple(name: &str) -> Layout {
    layout(name, "2026-01-01T00:00:00Z", &["1-1-1"])
}

fn files(store: &ProfileStore) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(store.dir())
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    names.sort();
    names
}

#[test]
fn path_is_the_directory_plus_name_dot_json() {
    let store = ProfileStore::new("/tmp/profiles");
    assert_eq!(store.dir(), PathBuf::from("/tmp/profiles"));
    assert_eq!(store.path("home"), PathBuf::from("/tmp/profiles/home.json"));
    assert_eq!(store.path("a.b"), PathBuf::from("/tmp/profiles/a.b.json"));
}

#[test]
fn save_creates_the_directory_and_load_round_trips() {
    let (_dir, store) = temp_store();
    assert!(!store.exists("home"));
    store.save(&simple("home")).unwrap();
    assert!(store.exists("home"));
    assert_eq!(store.load("home").unwrap(), simple("home"));
    assert_eq!(files(&store), ["home.json"]);
}

#[test]
fn save_replaces_an_existing_profile() {
    let (_dir, store) = temp_store();
    store.save(&simple("home")).unwrap();
    let newer = layout("home", "2027-01-01T00:00:00Z", &["9-9-9"]);
    store.save(&newer).unwrap();
    assert_eq!(store.load("home").unwrap(), newer);
    assert_eq!(files(&store), ["home.json"]);
}

#[test]
fn the_file_name_decides_the_profile_name() {
    let (_dir, store) = temp_store();
    store.save(&simple("original")).unwrap();
    fs::rename(store.path("original"), store.path("renamed in finder")).unwrap();
    assert_eq!(
        store.load("renamed in finder").unwrap().profile,
        "renamed in finder"
    );
}

#[test]
fn load_reports_missing_and_damaged_files() {
    let (_dir, store) = temp_store();
    assert_eq!(store.load("nope"), Err(StoreError::NotFound("nope".into())));
    store.save(&simple("home")).unwrap();
    fs::write(store.path("home"), "{ not json").unwrap();
    assert_eq!(store.load("home"), Err(StoreError::Corrupt("home".into())));
}

#[test]
fn names_are_sorted_ignoring_case_and_only_json_files_count() {
    let (_dir, store) = temp_store();
    for name in ["gamma", "Beta", "alpha"] {
        store.save(&simple(name)).unwrap();
    }
    fs::write(store.dir().join("notes.txt"), "x").unwrap();
    fs::write(store.dir().join(".half-written.json.tmp"), "x").unwrap();
    assert_eq!(store.names(), ["alpha", "Beta", "gamma"]);
}

#[test]
fn names_of_a_missing_directory_is_empty() {
    let (_dir, store) = temp_store();
    assert!(store.names().is_empty());
    assert!(store.layouts().is_empty());
}

#[test]
fn save_fails_when_the_directory_cannot_be_created() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("file"), "x").unwrap();
    let store = ProfileStore::new(dir.path().join("file").join("sub"));
    assert!(matches!(
        store.save(&simple("home")),
        Err(StoreError::Io(_))
    ));
}

#[test]
fn save_fails_when_the_temporary_file_cannot_be_written() {
    let (_dir, store) = temp_store();
    fs::create_dir_all(store.dir().join(".home.json.tmp")).unwrap();
    assert!(matches!(
        store.save(&simple("home")),
        Err(StoreError::Io(_))
    ));
    assert!(!store.exists("home"));
}

#[test]
fn save_fails_when_the_profile_file_cannot_be_replaced() {
    let (_dir, store) = temp_store();
    fs::create_dir_all(store.path("home").join("in the way")).unwrap();
    assert!(matches!(
        store.save(&simple("home")),
        Err(StoreError::Io(_))
    ));
}

#[test]
fn rename_moves_the_profile() {
    let (_dir, store) = temp_store();
    store.save(&simple("home")).unwrap();
    store.rename("home", "house").unwrap();
    assert_eq!(store.names(), ["house"]);
    assert_eq!(store.load("house").unwrap(), simple("house"));
}

#[test]
fn rename_of_a_missing_profile_is_not_found() {
    let (_dir, store) = temp_store();
    store.save(&simple("other")).unwrap();
    assert_eq!(
        store.rename("nope", "other"),
        Err(StoreError::NotFound("nope".into()))
    );
}

#[test]
fn rename_onto_another_profile_is_refused() {
    let (_dir, store) = temp_store();
    store.save(&simple("home")).unwrap();
    let office = layout("office", "2026-02-02T00:00:00Z", &["2-2-2"]);
    store.save(&office).unwrap();
    assert_eq!(
        store.rename("home", "office"),
        Err(StoreError::Exists("office".into()))
    );
    assert_eq!(store.load("home").unwrap(), simple("home"));
    assert_eq!(store.load("office").unwrap(), office);
}

#[test]
fn rename_can_change_only_the_letter_case() {
    let (_dir, store) = temp_store();
    store.save(&simple("home")).unwrap();
    store.rename("home", "Home").unwrap();
    assert_eq!(store.names(), ["Home"]);
}

#[test]
fn rename_reports_other_failures() {
    let (_dir, store) = temp_store();
    store.save(&simple("home")).unwrap();
    assert!(matches!(
        store.rename("home", "no-such-folder/x"),
        Err(StoreError::Io(_))
    ));
    assert!(store.exists("home"));
}

#[test]
fn delete_removes_the_profile() {
    let (_dir, store) = temp_store();
    store.save(&simple("home")).unwrap();
    store.delete("home").unwrap();
    assert!(!store.exists("home"));
    assert_eq!(
        store.delete("home"),
        Err(StoreError::NotFound("home".into()))
    );
}

#[test]
fn layouts_skips_damaged_files() {
    let (_dir, store) = temp_store();
    store.save(&simple("good")).unwrap();
    store.save(&simple("bad")).unwrap();
    fs::write(store.path("bad"), "garbage").unwrap();
    assert_eq!(store.layouts(), [simple("good")]);
}

#[test]
fn profile_for_picks_the_newest_profile_saved_with_these_displays() {
    let (_dir, store) = temp_store();
    store
        .save(&layout("old", "2026-01-01T00:00:00Z", &["1-1-1", "2-2-2"]))
        .unwrap();
    store
        .save(&layout("new", "2026-06-01T00:00:00Z", &["2-2-2", "1-1-1"]))
        .unwrap();
    store
        .save(&layout("mid", "2026-03-01T00:00:00Z", &["1-1-1", "2-2-2"]))
        .unwrap();
    store
        .save(&layout(
            "zz other screens",
            "2027-01-01T00:00:00Z",
            &["3-3-3"],
        ))
        .unwrap();
    let connected = [display("2-2-2"), display("1-1-1")];
    assert_eq!(store.profile_for(&connected), Some("new".into()));
    assert_eq!(
        store.profile_for(&[display("3-3-3")]),
        Some("zz other screens".into())
    );
}

#[test]
fn profile_for_is_none_when_nothing_matches() {
    let (_dir, store) = temp_store();
    assert_eq!(store.profile_for(&[display("1-1-1")]), None);
    store.save(&simple("home")).unwrap();
    assert_eq!(store.profile_for(&[display("7-7-7")]), None);
    assert_eq!(store.profile_for(&[]), None);
}

#[test]
fn default_directory() {
    let home = || Some("/Users/me".to_string());
    assert_eq!(
        default_dir(Some("/custom".into()), home()),
        PathBuf::from("/custom")
    );
    assert_eq!(
        default_dir(None, home()),
        PathBuf::from("/Users/me/.config/layoutsmart")
    );
    assert_eq!(
        default_dir(Some(String::new()), home()),
        PathBuf::from("/Users/me/.config/layoutsmart")
    );
    assert_eq!(
        default_dir(None, None),
        PathBuf::from(".config/layoutsmart")
    );
}

#[test]
fn error_messages() {
    assert_eq!(
        StoreError::NotFound("x".into()).to_string(),
        "There is no saved profile named 'x'."
    );
    assert_eq!(
        StoreError::Exists("x".into()).to_string(),
        "A profile named 'x' already exists."
    );
    assert_eq!(
        StoreError::Corrupt("x".into()).to_string(),
        "Profile 'x' can't be read: its file is damaged."
    );
    assert_eq!(
        StoreError::Io("disk full".into()).to_string(),
        "Couldn't write the profile: disk full"
    );
}

#[test]
fn error_is_cloneable_and_printable() {
    let e = StoreError::NotFound("x".into());
    assert_eq!(e.clone(), e);
    assert_eq!(format!("{e:?}"), "NotFound(\"x\")");
}
