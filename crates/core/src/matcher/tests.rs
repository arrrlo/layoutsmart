use super::*;
use crate::desktops::{DesktopMap, Kind};
use crate::model::WindowInfo;
use crate::testing::{display, frame, screen};
use crate::windows::{LiveWindow, Unreached};

const MAIN: &str = "1-2-10";
const SIDE: &str = "1-2-20";

/// Main: desktop 1 (11), [desktop 2] (12), a full-screen app (13), desktop 3 (14).
/// Side: [desktop 1] (21), a full-screen app (22).
fn map() -> DesktopMap {
    DesktopMap::new(
        &[display("Main", 10, 0.0), display("Side", 20, 1920.0)],
        &[
            screen(10, 12, &[(11, false), (12, false), (13, true), (14, false)]),
            screen(20, 21, &[(21, false), (22, true)]),
        ],
    )
}

fn saved(app: &str, title: &str, x: f64) -> WindowInfo {
    WindowInfo::new(app, &format!("com.test.{app}"), title, frame(x))
}

/// An open, reachable window whose desktop is unknown.
fn open(id: u32, app: &str, title: &str, x: f64) -> Open {
    Open {
        id: Some(id),
        handle: Some(id as usize),
        app: app.into(),
        bundle_id: format!("com.test.{app}"),
        title: title.into(),
        frame: frame(x),
        fullscreen: false,
        place: None,
    }
}

/// The same window on the desktop with this id in `map()`.
fn on(space: u64, window: Open) -> Open {
    let place = map().place(&[space]).unwrap();
    Open {
        fullscreen: place.kind.is_fullscreen(),
        place: Some(place),
        ..window
    }
}

fn unreachable(window: Open) -> Open {
    Open {
        handle: None,
        ..window
    }
}

/// (saved title, id of the open window) for each pair.
fn paired(saved: &[WindowInfo], open: &[Open]) -> Vec<(String, u32)> {
    let saved: Vec<&WindowInfo> = saved.iter().collect();
    pair(&saved, open)
        .iter()
        .map(|pair| (pair.saved.title.clone(), pair.open.id.unwrap()))
        .collect()
}

fn titles(pairs: &[(&str, u32)]) -> Vec<(String, u32)> {
    pairs
        .iter()
        .map(|(title, id)| (title.to_string(), *id))
        .collect()
}

// ---- pairing

#[test]
fn a_window_pairs_with_the_saved_entry_of_the_same_app_and_title() {
    let saved = [saved("a", "one", 100.0), saved("a", "two", 200.0)];
    let open = [open(1, "a", "two", 0.0), open(2, "a", "one", 0.0)];
    assert_eq!(paired(&saved, &open), titles(&[("one", 2), ("two", 1)]));
}

#[test]
fn pairs_come_in_the_order_of_the_profile_and_say_which_entry_they_are() {
    let saved = [
        saved("a", "one", 100.0),
        saved("b", "gone", 0.0),
        saved("a", "two", 200.0),
    ];
    let open = [open(1, "a", "two", 0.0), open(2, "a", "one", 0.0)];
    let refs: Vec<&WindowInfo> = saved.iter().collect();
    let pairs = pair(&refs, &open);
    let orders: Vec<usize> = pairs.iter().map(|pair| pair.order).collect();
    assert_eq!(orders, [0, 2]);
    assert_eq!(pairs[1].saved, &saved[2]);
    assert_eq!(pairs[1].open, &open[0]);
}

#[test]
fn windows_whose_titles_changed_are_paired_in_order() {
    let saved = [saved("a", "old 1", 100.0), saved("a", "old 2", 200.0)];
    let open = [open(1, "a", "new 1", 0.0), open(2, "a", "new 2", 0.0)];
    assert_eq!(paired(&saved, &open), titles(&[("old 1", 1), ("old 2", 2)]));
}

#[test]
fn a_title_match_claims_its_entry_before_leftovers_are_paired() {
    let saved = [saved("a", "keep", 100.0), saved("a", "old", 200.0)];
    let open = [open(1, "a", "new", 0.0), open(2, "a", "keep", 0.0)];
    assert_eq!(paired(&saved, &open), titles(&[("keep", 2), ("old", 1)]));
}

#[test]
fn duplicate_titles_are_matched_one_to_one() {
    let saved = [saved("a", "zsh", 100.0), saved("a", "zsh", 200.0)];
    let open = [
        open(1, "a", "zsh", 0.0),
        open(2, "a", "zsh", 0.0),
        open(3, "a", "zsh", 0.0),
    ];
    let refs: Vec<&WindowInfo> = saved.iter().collect();
    let pairs = pair(&refs, &open);
    let found: Vec<(f64, u32)> = pairs
        .iter()
        .map(|pair| (pair.saved.x, pair.open.id.unwrap()))
        .collect();
    assert_eq!(found, [(100.0, 1), (200.0, 2)]);
}

#[test]
fn other_apps_are_never_paired() {
    let saved = [saved("a", "doc", 100.0)];
    assert!(paired(&saved, &[open(1, "b", "doc", 0.0)]).is_empty());
    assert!(paired(&saved, &[]).is_empty());
    assert!(paired(&[], &[open(1, "b", "doc", 0.0)]).is_empty());
}

#[test]
fn among_windows_with_the_same_title_the_one_on_the_saved_desktop_is_preferred() {
    let saved = [
        saved("a", "zsh", 100.0).on_desktop(MAIN, 1),
        saved("a", "zsh", 200.0).on_desktop(MAIN, 3),
    ];
    // Listed the other way round.
    let open = [
        on(14, open(1, "a", "zsh", 0.0)),
        on(11, open(2, "a", "zsh", 0.0)),
    ];
    let refs: Vec<&WindowInfo> = saved.iter().collect();
    let found: Vec<(f64, u32)> = pair(&refs, &open)
        .iter()
        .map(|pair| (pair.saved.x, pair.open.id.unwrap()))
        .collect();
    assert_eq!(found, [(100.0, 2), (200.0, 1)]);
}

#[test]
fn a_title_match_on_another_desktop_beats_an_untitled_match_on_the_same_one() {
    let saved = [saved("a", "notes", 100.0).on_desktop(MAIN, 1)];
    let open = [
        on(11, open(1, "a", "something else", 0.0)),
        on(14, open(2, "a", "notes", 0.0)),
    ];
    assert_eq!(paired(&saved, &open), titles(&[("notes", 2)]));
}

#[test]
fn leftovers_are_paired_within_their_desktop_before_across_desktops() {
    let saved = [
        saved("a", "old 1", 100.0).on_desktop(MAIN, 1),
        saved("a", "old 2", 200.0).on_desktop(MAIN, 3),
        saved("a", "old 3", 300.0).on_desktop(SIDE, 1),
    ];
    let open = [
        on(14, open(1, "a", "new x", 0.0)),
        on(12, open(2, "a", "new y", 0.0)),
        on(11, open(3, "a", "new z", 0.0)),
    ];
    // 1 and 3 are on desktops the profile knows; 2 takes what is left.
    assert_eq!(
        paired(&saved, &open),
        titles(&[("old 1", 3), ("old 2", 1), ("old 3", 2)])
    );
}

#[test]
fn the_same_desktop_number_on_another_screen_is_not_the_same_desktop() {
    let saved = [
        saved("a", "old 1", 100.0).on_desktop(SIDE, 1),
        saved("a", "old 2", 200.0).on_desktop(MAIN, 1),
    ];
    let open = [on(11, open(1, "a", "new", 0.0))];
    assert_eq!(paired(&saved, &open), titles(&[("old 2", 1)]));
}

#[test]
fn full_screen_on_a_screen_counts_as_a_desktop_of_its_own() {
    let saved = [
        saved("a", "old 1", 100.0).on_desktop(MAIN, 2),
        saved("a", "old 2", 200.0).in_full_screen(MAIN),
    ];
    let here = [
        on(13, open(1, "a", "new x", 0.0)),
        on(12, open(2, "a", "new y", 0.0)),
    ];
    assert_eq!(paired(&saved, &here), titles(&[("old 1", 2), ("old 2", 1)]));
    // A full-screen window on the other screen is not "the same desktop".
    let elsewhere = [
        on(22, open(1, "a", "new x", 0.0)),
        on(13, open(2, "a", "new y", 0.0)),
    ];
    assert_eq!(
        paired(&saved, &elsewhere),
        titles(&[("old 1", 1), ("old 2", 2)])
    );
}

#[test]
fn a_window_on_all_desktops_matches_any_desktop_of_its_screen() {
    let saved = [
        saved("a", "old 1", 100.0).on_desktop(MAIN, 3),
        saved("a", "old 2", 200.0).on_display(MAIN),
    ];
    let mut everywhere = on(12, open(1, "a", "new x", 0.0));
    everywhere.place.as_mut().unwrap().everywhere = true;
    assert_eq!(paired(&saved, &[everywhere]), titles(&[("old 1", 1)]));
    // An entry saved as "on all desktops" takes any ordinary window of its screen.
    let open = [
        on(21, open(1, "a", "new x", 0.0)),
        on(11, open(2, "a", "new y", 0.0)),
    ];
    assert_eq!(paired(&saved[1..], &open), titles(&[("old 2", 2)]));
}

// ---- what a pair needs

fn step_of(saved: WindowInfo, open: Open) -> Option<Step> {
    let saved = [&saved];
    let open = [open];
    pair(&saved, &open)[0].step(&map())
}

#[test]
fn a_window_in_place_needs_nothing() {
    let entry = saved("a", "t", 100.0).on_desktop(MAIN, 2);
    assert_eq!(
        step_of(entry.clone(), on(12, open(1, "a", "t", 100.0))),
        None
    );
    // Fractions of a point do not count.
    assert_eq!(step_of(entry, on(12, open(1, "a", "t", 100.3))), None);
    // Old profiles know nothing about desktops: only the frame counts.
    let old = saved("a", "t", 100.0);
    assert_eq!(step_of(old.clone(), on(14, open(1, "a", "t", 100.0))), None);
    assert_eq!(step_of(old, open(1, "a", "t", 100.0)), None);
}

#[test]
fn a_window_with_the_wrong_frame_desktop_or_screen_is_to_be_placed() {
    let entry = saved("a", "t", 100.0).on_desktop(MAIN, 2);
    for (space, x) in [(12, 0.0), (11, 100.0), (21, 100.0)] {
        assert_eq!(
            step_of(entry.clone(), on(space, open(1, "a", "t", x))),
            Some(Step::Place),
            "{space} {x}"
        );
    }
    let old = saved("a", "t", 100.0);
    assert_eq!(step_of(old, open(1, "a", "t", 0.0)), Some(Step::Place));
}

#[test]
fn full_screen_windows_enter_leave_or_stay() {
    let full = saved("a", "t", 0.0).in_full_screen(MAIN);
    assert_eq!(step_of(full.clone(), on(13, open(1, "a", "t", 5.0))), None);
    // Full screen, but on the other screen: it has to come out first.
    assert_eq!(
        step_of(full.clone(), on(22, open(1, "a", "t", 0.0))),
        Some(Step::Leave)
    );
    assert_eq!(
        step_of(full, on(12, open(1, "a", "t", 0.0))),
        Some(Step::Enter)
    );
    let normal = saved("a", "t", 100.0).on_desktop(MAIN, 2);
    assert_eq!(
        step_of(normal, on(13, open(1, "a", "t", 100.0))),
        Some(Step::Leave)
    );
}

#[test]
fn a_screen_that_is_not_connected_is_not_asked_for() {
    let normal = saved("a", "t", 100.0).on_desktop("9-9-9", 2);
    assert_eq!(
        step_of(normal.clone(), on(21, open(1, "a", "t", 100.0))),
        None
    );
    assert_eq!(
        step_of(normal, on(21, open(1, "a", "t", 0.0))),
        Some(Step::Place)
    );
    let full = saved("a", "t", 0.0).in_full_screen("9-9-9");
    assert_eq!(step_of(full, on(22, open(1, "a", "t", 0.0))), None);
}

#[test]
fn a_desktop_that_no_longer_exists_means_the_last_one() {
    // Main has three ordinary desktops; the profile says 5.
    let entry = saved("a", "t", 100.0).on_desktop(MAIN, 5);
    assert_eq!(
        step_of(entry.clone(), on(14, open(1, "a", "t", 100.0))),
        None
    );
    assert_eq!(
        step_of(entry.clone(), on(12, open(1, "a", "t", 100.0))),
        Some(Step::Place)
    );
    let saved = [&entry];
    let open = [on(12, open(1, "a", "t", 100.0))];
    assert_eq!(pair(&saved, &open)[0].home(&map()), Some((MAIN.into(), 3)));
}

#[test]
fn a_window_that_cannot_be_reached_is_reached_for_only_when_it_needs_something() {
    let entry = saved("a", "t", 100.0).on_desktop(MAIN, 3);
    assert_eq!(
        step_of(entry.clone(), unreachable(on(14, open(1, "a", "", 100.0)))),
        None
    );
    assert_eq!(
        step_of(entry, unreachable(on(11, open(1, "a", "", 100.0)))),
        Some(Step::Reach)
    );
}

#[test]
fn a_pair_knows_its_screen_and_the_desktop_to_bring_into_view() {
    let entry = saved("a", "t", 100.0).on_desktop(MAIN, 3);
    let saved_list = [&entry];

    // On the right screen, wrong desktop.
    let open_list = [on(11, open(1, "a", "t", 100.0))];
    let pairs = pair(&saved_list, &open_list);
    assert_eq!(
        pairs[0].screen(&map()).map(|row| row.name.as_str()),
        Some("Main")
    );
    assert!(pairs[0].on_screen(&map()));
    assert_eq!(pairs[0].home(&map()), Some((MAIN.into(), 3)));

    // On another screen.
    let open_list = [on(21, open(1, "a", "t", 100.0))];
    let pairs = pair(&saved_list, &open_list);
    assert!(!pairs[0].on_screen(&map()));
    assert_eq!(pairs[0].home(&map()), Some((MAIN.into(), 3)));

    // Already there: nothing to bring into view.
    let open_list = [on(14, open(1, "a", "t", 0.0))];
    let pairs = pair(&saved_list, &open_list);
    assert_eq!(pairs[0].home(&map()), None);

    // No desktop recorded: the screen still counts, but no desktop is asked for.
    let loose = saved("a", "t", 100.0).on_display(MAIN);
    let saved_list = [&loose];
    let open_list = [on(21, open(1, "a", "t", 100.0))];
    let pairs = pair(&saved_list, &open_list);
    assert!(!pairs[0].on_screen(&map()));
    assert_eq!(pairs[0].home(&map()), None);
    assert_eq!(pairs[0].step(&map()), Some(Step::Place));

    // A window whose place is unknown is on no screen.
    let open_list = [open(1, "a", "t", 100.0)];
    let pairs = pair(&saved_list, &open_list);
    assert!(!pairs[0].on_screen(&map()));
}

// ---- open windows

fn live() -> LiveWindow {
    LiveWindow {
        handle: 4,
        window_id: Some(77),
        app: "Notes".into(),
        bundle_id: "com.test.notes".into(),
        title: "List".into(),
        frame: frame(30.0),
        fullscreen: true,
        place: map().place(&[13]),
    }
}

#[test]
fn an_open_window_is_made_from_a_reachable_or_an_unreachable_one() {
    assert_eq!(
        Open::reached(&live()),
        Open {
            id: Some(77),
            handle: Some(4),
            app: "Notes".into(),
            bundle_id: "com.test.notes".into(),
            title: "List".into(),
            frame: frame(30.0),
            fullscreen: true,
            place: map().place(&[13]),
        }
    );
    let hidden = Unreached {
        window_id: 78,
        pid: 5,
        app: "Notes".into(),
        bundle_id: "com.test.notes".into(),
        frame: frame(40.0),
        place: map().place(&[14]).unwrap(),
    };
    assert_eq!(
        Open::unreached(&hidden),
        Open {
            id: Some(78),
            handle: None,
            app: "Notes".into(),
            bundle_id: "com.test.notes".into(),
            title: String::new(),
            frame: frame(40.0),
            fullscreen: false,
            place: map().place(&[14]),
        }
    );
    let full = Unreached {
        place: map().place(&[22]).unwrap(),
        ..hidden
    };
    assert!(Open::unreached(&full).fullscreen);
}

#[test]
fn an_open_window_is_recorded_with_its_screen_and_desktop() {
    let plain = open(1, "a", "t", 100.0);
    assert_eq!(plain.to_saved(), saved("a", "t", 100.0));
    assert_eq!(
        on(14, plain.clone()).to_saved(),
        saved("a", "t", 100.0).on_desktop(MAIN, 3)
    );
    assert_eq!(
        on(22, plain.clone()).to_saved(),
        saved("a", "t", 100.0).in_full_screen(SIDE)
    );
    let mut everywhere = on(12, plain);
    everywhere.place.as_mut().unwrap().everywhere = true;
    assert_eq!(
        everywhere.to_saved(),
        saved("a", "t", 100.0).on_display(MAIN)
    );
}

#[test]
fn full_screen_windows_are_listed_last_in_the_order_of_their_desktops() {
    let listed = vec![
        on(22, open(1, "a", "side full", 0.0)),
        on(13, open(2, "a", "main full", 0.0)),
        on(14, open(3, "a", "normal", 0.0)),
        open(4, "a", "unknown place", 0.0),
        on(11, open(5, "a", "first desktop", 0.0)),
    ];
    let ids: Vec<u32> = in_order(listed)
        .iter()
        .map(|window| window.id.unwrap())
        .collect();
    // Side's full-screen desktop is second in its row, Main's is third.
    assert_eq!(ids, [3, 4, 5, 1, 2]);

    // A full-screen desktop that is first in its row still comes after ordinary windows.
    let map = DesktopMap::new(
        &[display("Main", 10, 0.0)],
        &[screen(10, 31, &[(31, true), (32, false)])],
    );
    let first = Open {
        fullscreen: true,
        place: map.place(&[31]),
        ..open(1, "a", "full", 0.0)
    };
    assert_eq!(first.place.as_ref().unwrap().kind, Kind::FullScreen);
    let second = Open {
        place: map.place(&[32]),
        ..open(2, "a", "normal", 0.0)
    };
    let ids: Vec<u32> = in_order(vec![first, second])
        .iter()
        .map(|window| window.id.unwrap())
        .collect();
    assert_eq!(ids, [2, 1]);
}

#[test]
fn within_that_windows_are_listed_oldest_first_whatever_order_they_arrive_in() {
    // Which windows macOS reports first depends on which desktop is in view; the window
    // number does not.
    let listed = vec![
        on(12, open(9, "a", "in view, newest", 0.0)),
        on(22, open(8, "a", "full", 0.0)),
        on(22, open(2, "a", "full, older", 0.0)),
        on(14, open(4, "a", "hidden, older", 0.0)),
        on(11, open(6, "a", "hidden", 0.0)),
    ];
    let ids: Vec<u32> = in_order(listed)
        .iter()
        .map(|window| window.id.unwrap())
        .collect();
    assert_eq!(ids, [4, 6, 9, 2, 8]);

    // Windows without a number keep the order they came in.
    let nameless = |title: &str| Open {
        id: None,
        ..open(0, "a", title, 0.0)
    };
    let titles: Vec<String> = in_order(vec![nameless("b"), nameless("a")])
        .into_iter()
        .map(|window| window.title)
        .collect();
    assert_eq!(titles, ["b", "a"]);
}

#[test]
fn types_are_cloneable_and_printable() {
    let window = open(1, "a", "t", 0.0);
    assert_eq!(window.clone(), window);
    assert!(format!("{window:?}").starts_with("Open"));
    let entry = saved("a", "t", 0.0);
    let saved = [&entry];
    let open = [window];
    let pairs = pair(&saved, &open);
    assert_eq!(pairs[0].clone(), pairs[0]);
    assert!(format!("{:?}", pairs[0]).starts_with("Pair"));
    assert_eq!(format!("{:?}", Step::Reach.clone()), "Reach");
}
