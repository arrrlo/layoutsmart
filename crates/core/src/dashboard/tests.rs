use super::*;
use crate::flows::Session;
use crate::geometry::{Rect, Size};
use crate::model::{DisplayInfo, Layout, SavedScreen, Showing, WindowInfo};
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
    let shapes = shapes(&layout(two_screens(), vec![]), CANVAS, &[]);
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
    let shapes = shapes(&layout(tall, vec![]), CANVAS, &[]);
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
        &[],
    );
    // 500 x 800 available → scale 0.5 → 500 x 250, centred vertically.
    assert_rect(shapes[0].rect, 20.0, 295.0, 500.0, 250.0);
}

#[test]
fn windows_are_drawn_where_they_sit_on_their_screen() {
    let windows = vec![win("Safari", "Apple", 100.0, 50.0, 800.0, 600.0)];
    let shapes = shapes(&layout(two_screens(), windows), CANVAS, &[]);
    assert_eq!(shapes.len(), 3);
    assert_eq!(shapes[2].kind, ShapeKind::Window);
    assert_rect(shapes[2].rect, 45.0, 177.5, 200.0, 150.0);
    assert_eq!(shapes[2].label, "Safari");
    assert_eq!(shapes[2].detail, "Apple");
}

#[test]
fn a_window_across_two_screens_is_shown_on_the_one_holding_most_of_it() {
    let windows = vec![win("Notes", "n", 1800.0, 100.0, 400.0, 300.0)];
    let shapes = shapes(&layout(two_screens(), windows), CANVAS, &[]);
    assert_rect(shapes[2].rect, 500.0, 190.0, 70.0, 75.0);

    let mostly_left = vec![win("Notes", "n", 1600.0, 100.0, 400.0, 300.0)];
    let shapes = super::shapes(&layout(two_screens(), mostly_left), CANVAS, &[]);
    assert_rect(shapes[2].rect, 420.0, 190.0, 80.0, 75.0);
}

#[test]
fn windows_outside_every_screen_are_left_out() {
    let windows = vec![
        win("Lost", "far away", 5000.0, 0.0, 100.0, 100.0),
        win("Edge", "touching only", 3840.0, 0.0, 100.0, 100.0),
    ];
    assert_eq!(
        shapes(&layout(two_screens(), windows), CANVAS, &[]).len(),
        2
    );
}

#[test]
fn rear_windows_are_drawn_first_so_front_windows_end_up_on_top() {
    let windows = vec![
        win("Front", "f", 0.0, 0.0, 100.0, 100.0),
        win("Back", "b", 0.0, 0.0, 100.0, 100.0),
    ];
    let shapes = shapes(&layout(two_screens(), windows), CANVAS, &[]);
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
    let shapes = shapes(&layout(two_screens(), windows), CANVAS, &[]);
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
    assert!(shapes(&layout(vec![], windows.clone()), CANVAS, &[]).is_empty());

    let flat = vec![screen("Flat", 0.0, 0.0, 1920.0, 0.0)];
    assert!(shapes(&layout(flat, windows.clone()), CANVAS, &[]).is_empty());
    let thin = vec![screen("Thin", 0.0, 0.0, 0.0, 1080.0)];
    assert!(shapes(&layout(thin, windows.clone()), CANVAS, &[]).is_empty());

    let narrow = Size {
        width: 2.0 * PADDING,
        height: 600.0,
    };
    assert!(shapes(&layout(two_screens(), windows.clone()), narrow, &[]).is_empty());
    let short = Size {
        width: 1000.0,
        height: 2.0 * PADDING,
    };
    assert!(shapes(&layout(two_screens(), windows.clone()), short, &[]).is_empty());

    let just_enough = Size {
        width: 2.0 * PADDING + 1.0,
        height: 2.0 * PADDING + 1.0,
    };
    assert_eq!(
        shapes(&layout(two_screens(), windows), just_enough, &[]).len(),
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
    let shapes = shapes(&layout(two_screens(), vec![]), CANVAS, &[]);
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

// ---- desktops

/// Left: three desktops, the second in view, and Mail full screen.
/// Right: one desktop and two full-screen Notes windows, the second of them in view.
fn desk_layout() -> Layout {
    let whole = |x: f64| (x, 0.0, 1920.0, 1080.0);
    let (lx, ly, lw, lh) = whole(0.0);
    let (rx, ry, rw, rh) = whole(1920.0);
    let mut profile = layout(
        two_screens(),
        vec![
            win("Safari", "one", 100.0, 100.0, 800.0, 600.0).on_desktop("Left", 1),
            win("Safari", "two", 200.0, 100.0, 800.0, 600.0).on_desktop("Left", 2),
            win("Clock", "", 300.0, 100.0, 400.0, 300.0).on_display("Left"),
            win("Mail", "inbox", lx, ly, lw, lh).in_full_screen("Left"),
            win("Term", "zsh", 2000.0, 100.0, 800.0, 600.0).on_desktop("Right", 1),
            win("Notes", "first", rx, ry, rw, rh).in_full_screen("Right"),
            win("Notes", "second", rx, ry, rw, rh).in_full_screen("Right"),
            win("Old", "no place", 2100.0, 200.0, 300.0, 300.0),
        ],
    );
    profile.desktops = vec![
        SavedScreen {
            display: "Left".into(),
            desktops: 3,
            showing: Showing::Desktop(2),
        },
        SavedScreen {
            display: "Right".into(),
            desktops: 1,
            showing: Showing::FullScreen {
                bundle_id: "com.Notes".into(),
                title: "second".into(),
            },
        },
    ];
    profile
}

fn chosen(pairs: &[(&str, usize)]) -> Vec<(String, usize)> {
    pairs
        .iter()
        .map(|(display, tab)| (display.to_string(), *tab))
        .collect()
}

/// Titles of the windows drawn, in drawing order.
fn drawn(profile: &Layout, picks: &[(&str, usize)]) -> Vec<String> {
    let rows = tab_rows(profile, &chosen(picks));
    shapes(profile, CANVAS, &rows)
        .into_iter()
        .filter(|shape| shape.kind == ShapeKind::Window)
        .map(|shape| shape.detail)
        .collect()
}

fn selected(profile: &Layout, picks: &[(&str, usize)]) -> Vec<usize> {
    tab_rows(profile, &chosen(picks))
        .iter()
        .map(|row| row.selected)
        .collect()
}

#[test]
fn each_screen_has_a_tab_per_desktop_and_per_full_screen_window() {
    let text = |words: &[&str]| -> Vec<String> { words.iter().map(|w| w.to_string()).collect() };
    assert_eq!(
        tab_rows(&desk_layout(), &[]),
        [
            TabRow {
                display: "Left".into(),
                name: "Left".into(),
                tabs: text(&["Desktop 1", "Desktop 2", "Desktop 3", "Mail"]),
                desktops: 3,
                selected: 1,
            },
            TabRow {
                display: "Right".into(),
                name: "Right".into(),
                tabs: text(&["Desktop 1", "Notes", "Notes"]),
                desktops: 1,
                selected: 2,
            },
        ]
    );
}

#[test]
fn a_profile_that_knows_nothing_about_desktops_has_no_tabs() {
    let old = layout(two_screens(), vec![win("a", "t", 0.0, 0.0, 10.0, 10.0)]);
    assert_eq!(tab_rows(&old, &[]), []);
    // Nor has a screen without desktops or full-screen windows.
    let mut empty = desk_layout();
    empty.desktops[0].desktops = 0;
    empty.windows.remove(3);
    let rows = tab_rows(&empty, &[]);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].display, "Right");
}

#[test]
fn a_screen_opens_on_what_it_showed_when_the_profile_was_saved() {
    let mut profile = desk_layout();
    assert_eq!(selected(&profile, &[]), [1, 2]);

    // The full-screen window's title has changed: the app's first full-screen window.
    profile.windows[6].title = "renamed".into();
    assert_eq!(selected(&profile, &[]), [1, 1]);
    // No full-screen window of that app is left: the first tab.
    profile.windows[5].bundle_id = "com.Other".into();
    profile.windows[6].bundle_id = "com.Other".into();
    assert_eq!(selected(&profile, &[]), [1, 0]);

    profile.desktops[0].showing = Showing::Unknown;
    profile.desktops[1].showing = Showing::Desktop(1);
    assert_eq!(selected(&profile, &[]), [0, 0]);
    // A desktop number beyond those the screen had means its last desktop.
    profile.desktops[0].showing = Showing::Desktop(9);
    assert_eq!(selected(&profile, &[]), [2, 0]);
    // A screen that had no ordinary desktop at all opens on its first tab.
    profile.desktops[0].desktops = 0;
    assert_eq!(selected(&profile, &[]), [0, 0]);
}

#[test]
fn a_chosen_tab_wins_and_one_that_does_not_exist_means_the_last() {
    let profile = desk_layout();
    assert_eq!(selected(&profile, &[("Left", 0)]), [0, 2]);
    assert_eq!(selected(&profile, &[("Right", 1)]), [1, 1]);
    assert_eq!(selected(&profile, &[("Left", 99)]), [3, 2]);
    assert_eq!(selected(&profile, &[("Elsewhere", 0)]), [1, 2]);
}

#[test]
fn only_windows_on_the_selected_desktops_are_drawn() {
    let profile = desk_layout();
    // As saved: Left shows desktop 2, Right shows the second Notes window full screen.
    // (A window whose place is unknown is always drawn; one on all desktops is drawn on
    // every ordinary desktop of its screen.)
    assert_eq!(drawn(&profile, &[]), ["no place", "second", "", "two"]);
    assert_eq!(
        drawn(&profile, &[("Left", 0), ("Right", 0)]),
        ["no place", "zsh", "", "one"]
    );
    assert_eq!(
        drawn(&profile, &[("Left", 2), ("Right", 1)]),
        ["no place", "first", ""]
    );
    // A full-screen tab shows that window alone — not even windows that are on all desktops.
    assert_eq!(
        drawn(&profile, &[("Left", 3)]),
        ["no place", "second", "inbox"]
    );
}

#[test]
fn a_window_on_a_screen_that_has_no_tabs_is_always_drawn() {
    let mut profile = desk_layout();
    profile.desktops.remove(1);
    assert_eq!(
        drawn(&profile, &[]),
        ["no place", "second", "first", "zsh", "", "two"]
    );
}

fn store_with(profile: &Layout, names: &[&str]) -> (tempfile::TempDir, ProfileStore) {
    let (dir, store) = temp_store();
    for name in names {
        let mut copy = profile.clone();
        copy.profile = name.to_string();
        store.save(&copy).unwrap();
    }
    (dir, store)
}

fn picked(dashboard: &Dashboard, store: &ProfileStore) -> Vec<usize> {
    dashboard
        .tab_rows(store)
        .iter()
        .map(|row| row.selected)
        .collect()
}

#[test]
fn clicking_a_tab_changes_what_the_picture_shows() {
    let (_dir, store) = store_with(&desk_layout(), &["a"]);
    let mut dashboard = Dashboard::open(&store);
    assert_eq!(picked(&dashboard, &store), [1, 2]);

    dashboard.choose(&store, 0, 0);
    dashboard.choose(&store, 1, 1);
    assert_eq!(picked(&dashboard, &store), [0, 1]);
    let titles: Vec<String> = dashboard
        .scene(&store, CANVAS)
        .shapes
        .into_iter()
        .filter(|shape| shape.kind == ShapeKind::Window)
        .map(|shape| shape.detail)
        .collect();
    assert_eq!(titles, ["no place", "first", "", "one"]);

    // Choosing again on the same screen replaces the earlier choice.
    dashboard.choose(&store, 0, 3);
    assert_eq!(picked(&dashboard, &store), [3, 1]);
    // A row or tab that does not exist is ignored.
    dashboard.choose(&store, 0, 0);
    dashboard.choose(&store, 5, 0);
    dashboard.choose(&store, 0, 4);
    assert_eq!(picked(&dashboard, &store), [0, 1]);
    // The last tab of a row can be chosen.
    dashboard.choose(&store, 1, 2);
    assert_eq!(picked(&dashboard, &store), [0, 2]);
}

#[test]
fn chosen_tabs_are_forgotten_when_another_profile_is_selected() {
    let (_dir, store) = store_with(&desk_layout(), &["a", "b"]);
    let mut dashboard = Dashboard::open(&store);
    dashboard.choose(&store, 0, 0);
    // Selecting the row that is already selected keeps them.
    dashboard.select(Some(0));
    assert_eq!(picked(&dashboard, &store), [0, 2]);
    dashboard.select(Some(1));
    assert_eq!(picked(&dashboard, &store), [1, 2]);
    dashboard.select(Some(0));
    assert_eq!(picked(&dashboard, &store), [1, 2]);
}

#[test]
fn chosen_tabs_survive_a_refresh_only_if_the_same_profile_stays_selected() {
    let (_dir, store) = store_with(&desk_layout(), &["a", "b"]);
    let mut dashboard = Dashboard::open(&store);
    dashboard.choose(&store, 0, 0);
    dashboard.refresh(&store, None);
    assert_eq!(picked(&dashboard, &store), [0, 2]);

    store.delete("a").unwrap();
    dashboard.refresh(&store, None);
    assert_eq!(dashboard.selected_name(), Some("b"));
    assert_eq!(picked(&dashboard, &store), [1, 2]);
}

#[test]
fn there_are_no_tabs_without_a_readable_profile() {
    let (_dir, store) = temp_store();
    let mut dashboard = Dashboard::open(&store);
    assert_eq!(dashboard.tab_rows(&store), []);
    dashboard.choose(&store, 0, 0);
    assert_eq!(dashboard.tab_rows(&store), []);

    given_profiles(&store, &["home"]);
    fs::write(store.path("home"), "{").unwrap();
    dashboard.refresh(&store, None);
    assert_eq!(dashboard.tab_rows(&store), []);
}

#[test]
fn a_tab_row_is_cloneable_and_printable() {
    let row = tab_rows(&desk_layout(), &[]).remove(0);
    assert_eq!(row.clone(), row);
    assert!(format!("{row:?}").starts_with("TabRow"));
}
