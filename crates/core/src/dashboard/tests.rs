use super::*;
use crate::flows::Session;
use crate::geometry::{Rect, Size};
use crate::model::{DisplayInfo, Layout, WindowInfo};
use crate::service::{self, Backend};
use crate::store::ProfileStore;
use crate::testing::Reply::{Cancel, No, Text, Yes};
use crate::testing::*;
use std::fs;

const CANVAS: Size = Size {
    width: 1000.0,
    height: 600.0,
};

fn screen(name: &str, x: f64, y: f64, width: f64, height: f64) -> DisplayInfo {
    DisplayInfo {
        name: name.into(),
        id: name.into(),
        x,
        y,
        width,
        height,
    }
}

fn win(app: &str, title: &str, x: f64, y: f64, width: f64, height: f64) -> WindowInfo {
    WindowInfo::new(
        app,
        &format!("com.{app}"),
        title,
        Rect::new(x, y, width, height),
    )
}

fn layout(displays: Vec<DisplayInfo>, windows: Vec<WindowInfo>) -> Layout {
    Layout {
        profile: "p".into(),
        saved_at: NOW_ISO.into(),
        displays,
        windows,
        desktops: Vec::new(),
    }
}

fn two_screens() -> Vec<DisplayInfo> {
    vec![
        screen("Left", 0.0, 0.0, 1920.0, 1080.0),
        screen("Right", 1920.0, 0.0, 1920.0, 1080.0),
    ]
}

fn assert_rect(actual: Rect, x: f64, y: f64, width: f64, height: f64) {
    let expected = [x, y, width, height];
    let got = [actual.x, actual.y, actual.width, actual.height];
    for (g, e) in got.iter().zip(expected) {
        assert!((g - e).abs() < 1e-9, "{got:?} != {expected:?}");
    }
}

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

fn given_profiles(store: &ProfileStore, names: &[&str]) {
    let sys = desk();
    let backend = Backend {
        sys: &sys,
        store,
        clock: &CLOCK,
    };
    for name in names {
        service::save(&backend, name).unwrap();
    }
}

// ---- the picture

#[test]
fn screens_are_scaled_to_fit_the_width_and_centred() {
    let shapes = shapes(&layout(two_screens(), vec![]), CANVAS);
    assert_eq!(shapes.len(), 2);
    assert_rect(shapes[0].rect, 20.0, 165.0, 480.0, 270.0);
    assert_rect(shapes[1].rect, 500.0, 165.0, 480.0, 270.0);
    assert_eq!(shapes[0].kind, ShapeKind::Display);
    assert_eq!(shapes[0].label, "Left");
    assert_eq!(shapes[0].detail, "1920 × 1080");
    assert_eq!(shapes[1].label, "Right");
    assert_eq!(shapes[0].color, 0);
}

#[test]
fn a_tall_arrangement_is_limited_by_height_and_may_start_off_origin() {
    let tall = vec![screen("Tall", -1000.0, -500.0, 1000.0, 2000.0)];
    let shapes = shapes(&layout(tall, vec![]), CANVAS);
    assert_rect(shapes[0].rect, 360.0, 20.0, 280.0, 560.0);
    assert_eq!(shapes[0].detail, "1000 × 2000");
}

#[test]
fn padding_is_kept_on_a_canvas_that_is_not_the_test_default() {
    let one = vec![screen("One", 100.0, 200.0, 1000.0, 500.0)];
    let shapes = shapes(
        &layout(one, vec![]),
        Size {
            width: 540.0,
            height: 840.0,
        },
    );
    // 500 x 800 available → scale 0.5 → 500 x 250, centred vertically.
    assert_rect(shapes[0].rect, 20.0, 295.0, 500.0, 250.0);
}

#[test]
fn windows_are_drawn_where_they_sit_on_their_screen() {
    let windows = vec![win("Safari", "Apple", 100.0, 50.0, 800.0, 600.0)];
    let shapes = shapes(&layout(two_screens(), windows), CANVAS);
    assert_eq!(shapes.len(), 3);
    assert_eq!(shapes[2].kind, ShapeKind::Window);
    assert_rect(shapes[2].rect, 45.0, 177.5, 200.0, 150.0);
    assert_eq!(shapes[2].label, "Safari");
    assert_eq!(shapes[2].detail, "Apple");
}

#[test]
fn a_window_across_two_screens_is_shown_on_the_one_holding_most_of_it() {
    let windows = vec![win("Notes", "n", 1800.0, 100.0, 400.0, 300.0)];
    let shapes = shapes(&layout(two_screens(), windows), CANVAS);
    assert_rect(shapes[2].rect, 500.0, 190.0, 70.0, 75.0);

    let mostly_left = vec![win("Notes", "n", 1600.0, 100.0, 400.0, 300.0)];
    let shapes = super::shapes(&layout(two_screens(), mostly_left), CANVAS);
    assert_rect(shapes[2].rect, 420.0, 190.0, 80.0, 75.0);
}

#[test]
fn windows_outside_every_screen_are_left_out() {
    let windows = vec![
        win("Lost", "far away", 5000.0, 0.0, 100.0, 100.0),
        win("Edge", "touching only", 3840.0, 0.0, 100.0, 100.0),
    ];
    assert_eq!(shapes(&layout(two_screens(), windows), CANVAS).len(), 2);
}

#[test]
fn rear_windows_are_drawn_first_so_front_windows_end_up_on_top() {
    let windows = vec![
        win("Front", "f", 0.0, 0.0, 100.0, 100.0),
        win("Back", "b", 0.0, 0.0, 100.0, 100.0),
    ];
    let shapes = shapes(&layout(two_screens(), windows), CANVAS);
    let labels: Vec<&str> = shapes.iter().map(|s| s.label.as_str()).collect();
    assert_eq!(labels, ["Left", "Right", "Back", "Front"]);
}

#[test]
fn each_app_gets_its_own_colour_in_order_of_appearance() {
    let mut windows = vec![
        win("Safari", "1", 0.0, 0.0, 10.0, 10.0),
        win("Mail", "2", 0.0, 0.0, 10.0, 10.0),
        win("Safari", "3", 0.0, 0.0, 10.0, 10.0),
    ];
    for n in 0..PALETTE_SIZE {
        windows.push(win(&format!("App{n}"), "x", 0.0, 0.0, 10.0, 10.0));
    }
    let shapes = shapes(&layout(two_screens(), windows), CANVAS);
    let colour = |label: &str, detail: &str| {
        shapes
            .iter()
            .find(|s| s.label == label && s.detail == detail)
            .unwrap()
            .color
    };
    assert_eq!(colour("Safari", "1"), 0);
    assert_eq!(colour("Mail", "2"), 1);
    assert_eq!(colour("Safari", "3"), 0);
    assert_eq!(colour("App0", "x"), 2);
    assert_eq!(colour("App5", "x"), PALETTE_SIZE - 1);
    // More apps than colours: start again from the first colour.
    assert_eq!(colour("App6", "x"), 0);
    assert_eq!(colour("App7", "x"), 1);
}

#[test]
fn nothing_is_drawn_without_usable_screens_or_room() {
    let windows = vec![win("Safari", "Apple", 0.0, 0.0, 10.0, 10.0)];
    assert!(shapes(&layout(vec![], windows.clone()), CANVAS).is_empty());

    let flat = vec![screen("Flat", 0.0, 0.0, 1920.0, 0.0)];
    assert!(shapes(&layout(flat, windows.clone()), CANVAS).is_empty());
    let thin = vec![screen("Thin", 0.0, 0.0, 0.0, 1080.0)];
    assert!(shapes(&layout(thin, windows.clone()), CANVAS).is_empty());

    let narrow = Size {
        width: 2.0 * PADDING,
        height: 600.0,
    };
    assert!(shapes(&layout(two_screens(), windows.clone()), narrow).is_empty());
    let short = Size {
        width: 1000.0,
        height: 2.0 * PADDING,
    };
    assert!(shapes(&layout(two_screens(), windows.clone()), short).is_empty());

    let just_enough = Size {
        width: 2.0 * PADDING + 1.0,
        height: 2.0 * PADDING + 1.0,
    };
    assert_eq!(
        shapes(&layout(two_screens(), windows), just_enough).len(),
        3
    );
}

// ---- the list

#[test]
fn opens_with_the_first_profile_selected() {
    let (_dir, store) = temp_store();
    given_profiles(&store, &["office", "home"]);
    let dashboard = Dashboard::open(&store);
    assert_eq!(dashboard.rows(), 2);
    assert_eq!(dashboard.row_title(0), "home");
    assert_eq!(dashboard.row_title(1), "office");
    assert_eq!(dashboard.row_title(2), "");
    assert_eq!(dashboard.selected_row(), Some(0));
    assert_eq!(dashboard.selected_name(), Some("home"));
    assert!(dashboard.can_edit());
}

#[test]
fn opens_empty_when_nothing_is_saved() {
    let (_dir, store) = temp_store();
    let dashboard = Dashboard::open(&store);
    assert_eq!(dashboard.rows(), 0);
    assert_eq!(dashboard.selected_row(), None);
    assert_eq!(dashboard.selected_name(), None);
    assert!(!dashboard.can_edit());
    assert_eq!(dashboard.row_title(0), "");
}

#[test]
fn selection_follows_clicks() {
    let (_dir, store) = temp_store();
    given_profiles(&store, &["home", "office"]);
    let mut dashboard = Dashboard::open(&store);
    dashboard.select(Some(1));
    assert_eq!(dashboard.selected_name(), Some("office"));
    dashboard.select(None);
    assert_eq!(dashboard.selected_name(), None);
    assert!(!dashboard.can_edit());
    dashboard.select(Some(1));
    dashboard.select(Some(2));
    assert_eq!(dashboard.selected_row(), None);
}

#[test]
fn summary_describes_the_selected_profile() {
    let (_dir, store) = temp_store();
    given_profiles(&store, &["home"]);
    let mut dashboard = Dashboard::open(&store);
    assert_eq!(
        dashboard.summary(&store),
        "2 displays · 3 windows · saved 2026-11-11 09:42 UTC"
    );
    dashboard.select(None);
    assert_eq!(dashboard.summary(&store), "");
}

#[test]
fn summary_uses_the_singular_and_reports_damaged_files() {
    let (_dir, store) = temp_store();
    let laptop = FakeSystem::new(
        vec![display("Built-in", 10, 0.0)],
        vec![window("Mail", "Inbox", 0.0)],
    );
    let backend = Backend {
        sys: &laptop,
        store: &store,
        clock: &CLOCK,
    };
    service::save(&backend, "solo").unwrap();
    assert_eq!(
        Dashboard::open(&store).summary(&store),
        "1 display · 1 window · saved 2026-11-11 09:42 UTC"
    );
    fs::write(store.path("solo"), "{").unwrap();
    assert_eq!(
        Dashboard::open(&store).summary(&store),
        "Profile 'solo' can't be read: its file is damaged."
    );
}

#[test]
fn scene_shows_the_selected_profile() {
    let (_dir, store) = temp_store();
    given_profiles(&store, &["home"]);
    let scene = Dashboard::open(&store).scene(&store, CANVAS);
    assert_eq!(scene.shapes.len(), 5);
    assert_eq!(scene.message, "");
}

#[test]
fn scene_explains_why_there_is_nothing_to_show() {
    let (_dir, store) = temp_store();
    let empty = Dashboard::open(&store).scene(&store, CANVAS);
    assert!(empty.shapes.is_empty());
    assert_eq!(empty.message, NO_PROFILES);

    given_profiles(&store, &["home"]);
    let mut dashboard = Dashboard::open(&store);
    dashboard.select(None);
    let unselected = dashboard.scene(&store, CANVAS);
    assert!(unselected.shapes.is_empty());
    assert_eq!(unselected.message, NOTHING_SELECTED);

    fs::write(store.path("home"), "{").unwrap();
    let damaged = Dashboard::open(&store).scene(&store, CANVAS);
    assert!(damaged.shapes.is_empty());
    assert_eq!(
        damaged.message,
        "Profile 'home' can't be read: its file is damaged."
    );
}

#[test]
fn renaming_keeps_the_renamed_profile_selected() {
    let (_dir, store) = temp_store();
    given_profiles(&store, &["home", "office"]);
    let (sys, ui) = (desk(), ScriptedUi::new(vec![Text("zebra")]));
    let mut dashboard = Dashboard::open(&store);
    dashboard.rename_selected(&session(&sys, &store, &ui));
    assert_eq!(dashboard.row_title(0), "office");
    assert_eq!(dashboard.row_title(1), "zebra");
    assert_eq!(dashboard.selected_name(), Some("zebra"));
    assert_eq!(dashboard.selected_row(), Some(1));
}

#[test]
fn a_cancelled_rename_changes_nothing() {
    let (_dir, store) = temp_store();
    given_profiles(&store, &["home", "office"]);
    let (sys, ui) = (desk(), ScriptedUi::new(vec![Cancel]));
    let mut dashboard = Dashboard::open(&store);
    dashboard.select(Some(1));
    dashboard.rename_selected(&session(&sys, &store, &ui));
    assert_eq!(dashboard.selected_name(), Some("office"));
    assert_eq!(dashboard.rows(), 2);
}

#[test]
fn deleting_selects_the_profile_that_takes_its_place() {
    let (_dir, store) = temp_store();
    given_profiles(&store, &["a", "b", "c"]);
    let sys = desk();
    let mut dashboard = Dashboard::open(&store);

    dashboard.select(Some(1));
    dashboard.delete_selected(&session(&sys, &store, &ScriptedUi::new(vec![Yes])));
    assert_eq!(dashboard.rows(), 2);
    assert_eq!(dashboard.selected_name(), Some("c"));

    dashboard.delete_selected(&session(&sys, &store, &ScriptedUi::new(vec![Yes])));
    assert_eq!(dashboard.selected_name(), Some("a"));

    dashboard.delete_selected(&session(&sys, &store, &ScriptedUi::new(vec![Yes])));
    assert_eq!(dashboard.rows(), 0);
    assert_eq!(dashboard.selected_row(), None);
}

#[test]
fn a_declined_delete_changes_nothing() {
    let (_dir, store) = temp_store();
    given_profiles(&store, &["a", "b"]);
    let (sys, ui) = (desk(), ScriptedUi::new(vec![No]));
    let mut dashboard = Dashboard::open(&store);
    dashboard.select(Some(1));
    dashboard.delete_selected(&session(&sys, &store, &ui));
    assert_eq!(dashboard.rows(), 2);
    assert_eq!(dashboard.selected_name(), Some("b"));
}

#[test]
fn rename_and_delete_do_nothing_without_a_selection() {
    let (_dir, store) = temp_store();
    given_profiles(&store, &["a"]);
    let (sys, ui) = (desk(), ScriptedUi::new(vec![Yes, Text("x")]));
    let mut dashboard = Dashboard::open(&store);
    dashboard.select(None);
    dashboard.rename_selected(&session(&sys, &store, &ui));
    dashboard.delete_selected(&session(&sys, &store, &ui));
    assert!(ui.log().is_empty());
    assert_eq!(store.names(), ["a"]);
    assert_eq!(dashboard.selected_row(), None);
}

#[test]
fn refresh_follows_the_selected_profile_when_the_list_changes() {
    let (_dir, store) = temp_store();
    given_profiles(&store, &["home", "office"]);
    let mut dashboard = Dashboard::open(&store);
    dashboard.select(Some(1));
    given_profiles(&store, &["aaa"]);
    dashboard.refresh(&store, None);
    assert_eq!(dashboard.rows(), 3);
    assert_eq!(dashboard.selected_name(), Some("office"));
    assert_eq!(dashboard.selected_row(), Some(2));
}

#[test]
fn refresh_can_be_told_which_profile_to_select() {
    let (_dir, store) = temp_store();
    given_profiles(&store, &["home", "office"]);
    let mut dashboard = Dashboard::open(&store);
    dashboard.refresh(&store, Some("office"));
    assert_eq!(dashboard.selected_name(), Some("office"));
    dashboard.refresh(&store, Some("gone"));
    assert_eq!(dashboard.selected_name(), Some("office"));
}

#[test]
fn refresh_keeps_the_row_when_the_selected_profile_disappears() {
    let (_dir, store) = temp_store();
    given_profiles(&store, &["a", "b", "c"]);
    let mut dashboard = Dashboard::open(&store);
    dashboard.select(Some(2));
    store.delete("c").unwrap();
    dashboard.refresh(&store, None);
    assert_eq!(dashboard.selected_name(), Some("b"));
    store.delete("a").unwrap();
    store.delete("b").unwrap();
    dashboard.refresh(&store, None);
    assert_eq!(dashboard.selected_row(), None);
}

#[test]
fn types_are_cloneable_and_printable() {
    let shapes = shapes(&layout(two_screens(), vec![]), CANVAS);
    assert_eq!(shapes.clone(), shapes);
    assert!(format!("{:?}", shapes[0]).starts_with("Shape"));
    let scene = Scene {
        shapes,
        message: String::new(),
    };
    assert_eq!(scene.clone(), scene);
    assert!(format!("{scene:?}").starts_with("Scene"));
    let (_dir, store) = temp_store();
    let dashboard = Dashboard::open(&store);
    assert_eq!(dashboard.clone(), dashboard);
    assert!(format!("{dashboard:?}").starts_with("Dashboard"));
    assert_eq!(ShapeKind::Display as usize, 0);
    assert_eq!(ShapeKind::Window as usize, 1);
}
