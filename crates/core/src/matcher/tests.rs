use super::*;
use crate::geometry::Rect;
use crate::model::WindowInfo;
use crate::windows::LiveWindow;

fn at(x: f64) -> Rect {
    Rect::new(x, 0.0, 100.0, 100.0)
}

fn saved(bundle: &str, title: &str, x: f64) -> WindowInfo {
    WindowInfo::new(bundle, bundle, title, at(x))
}

fn live(handle: usize, bundle: &str, title: &str, x: f64) -> LiveWindow {
    LiveWindow {
        handle,
        window_id: None,
        app: format!("{bundle} app"),
        bundle_id: bundle.into(),
        title: title.into(),
        frame: at(x),
        fullscreen: false,
        place: None,
    }
}

fn targets(moves: &[Move]) -> Vec<(usize, f64)> {
    moves.iter().map(|m| (m.handle, m.frame.x)).collect()
}

#[test]
fn window_with_the_same_title_is_moved_to_its_saved_frame() {
    let moves = plan_restore(&[saved("a", "doc", 500.0)], &[live(3, "a", "doc", 0.0)]);
    assert_eq!(
        moves,
        vec![Move {
            handle: 3,
            frame: at(500.0),
            app: "a app".into(),
            title: "doc".into(),
        }]
    );
}

#[test]
fn windows_already_in_place_are_skipped() {
    let moves = plan_restore(&[saved("a", "doc", 500.2)], &[live(0, "a", "doc", 500.0)]);
    assert!(moves.is_empty());
}

#[test]
fn title_match_wins_over_order() {
    let saved = [saved("a", "one", 100.0), saved("a", "two", 200.0)];
    let live = [live(0, "a", "two", 0.0), live(1, "a", "one", 0.0)];
    assert_eq!(
        targets(&plan_restore(&saved, &live)),
        [(0, 200.0), (1, 100.0)]
    );
}

#[test]
fn windows_with_changed_titles_are_paired_in_order() {
    let saved = [saved("a", "old 1", 100.0), saved("a", "old 2", 200.0)];
    let live = [live(0, "a", "new 1", 0.0), live(1, "a", "new 2", 0.0)];
    assert_eq!(
        targets(&plan_restore(&saved, &live)),
        [(0, 100.0), (1, 200.0)]
    );
}

#[test]
fn a_title_match_claims_its_entry_before_leftovers_are_paired() {
    let saved = [saved("a", "kept", 100.0), saved("a", "gone", 200.0)];
    let live = [live(0, "a", "renamed", 0.0), live(1, "a", "kept", 0.0)];
    assert_eq!(
        targets(&plan_restore(&saved, &live)),
        [(1, 100.0), (0, 200.0)]
    );
}

#[test]
fn duplicate_titles_are_matched_one_to_one() {
    let saved = [saved("a", "t", 100.0), saved("a", "t", 200.0)];
    let live = [live(0, "a", "t", 0.0), live(1, "a", "t", 0.0)];
    assert_eq!(
        targets(&plan_restore(&saved, &live)),
        [(0, 100.0), (1, 200.0)]
    );
}

#[test]
fn other_apps_are_never_matched() {
    let saved = [saved("a", "doc", 100.0)];
    let live = [live(0, "b", "doc", 0.0), live(1, "b", "other", 0.0)];
    assert!(plan_restore(&saved, &live).is_empty());
}

#[test]
fn extra_windows_without_a_saved_entry_are_left_alone() {
    let saved = [saved("a", "doc", 100.0)];
    let live = [
        live(0, "a", "doc", 0.0),
        live(1, "a", "extra", 0.0),
        live(2, "a", "extra 2", 0.0),
    ];
    assert_eq!(targets(&plan_restore(&saved, &live)), [(0, 100.0)]);
}

#[test]
fn nothing_saved_or_nothing_open_means_no_moves() {
    assert!(plan_restore(&[], &[live(0, "a", "doc", 0.0)]).is_empty());
    assert!(plan_restore(&[saved("a", "doc", 1.0)], &[]).is_empty());
}

#[test]
fn move_is_cloneable_and_printable() {
    let m = plan_restore(&[saved("a", "doc", 500.0)], &[live(3, "a", "doc", 0.0)]);
    assert_eq!(m.clone(), m);
    assert!(format!("{:?}", m[0]).starts_with("Move"));
}
