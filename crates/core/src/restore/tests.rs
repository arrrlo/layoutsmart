use super::testing::{FRONT_APP, Sim};
use super::*;
use crate::geometry::Rect;
use crate::model::{Layout, SavedScreen, Showing, WindowInfo};
use crate::service::{self, Backend, Error, Restored};
use crate::store::ProfileStore;
use crate::testing::{FakeSystem, FixedClock, NOW, display, temp_store, window};

const MAIN: &str = "1-2-10";
const SIDE: &str = "1-2-20";
const CLOCK: FixedClock = FixedClock(NOW);

/// On Main.
const F: Rect = Rect {
    x: 100.0,
    y: 80.0,
    width: 700.0,
    height: 500.0,
};
/// On Main, somewhere else.
const G: Rect = Rect {
    x: 300.0,
    y: 200.0,
    width: 640.0,
    height: 480.0,
};
/// On Side.
const S: Rect = Rect {
    x: 2100.0,
    y: -300.0,
    width: 600.0,
    height: 900.0,
};

/// Main (3 desktops, showing the 2nd) with Side to its right (1 desktop).
fn two_screens() -> Sim {
    Sim::new()
        .screen("Main", 10, Rect::new(0.0, 0.0, 2000.0, 1200.0), 3, 2)
        .screen("Side", 20, Rect::new(2000.0, -400.0, 1000.0, 1600.0), 1, 1)
}

fn one_screen() -> Sim {
    Sim::new().screen("Main", 10, Rect::new(0.0, 0.0, 2000.0, 1200.0), 3, 2)
}

/// A simulated Mac with a place to keep profiles.
struct Rig {
    sim: Sim,
    store: ProfileStore,
    _dir: tempfile::TempDir,
}

impl Rig {
    fn new(sim: Sim) -> Rig {
        let (_dir, store) = temp_store();
        Rig { sim, store, _dir }
    }

    fn backend(&self) -> Backend<'_> {
        Backend {
            sys: &self.sim,
            store: &self.store,
            clock: &CLOCK,
        }
    }

    /// Saves the desktop as profile "p" and returns what it looked like.
    fn save(&self) -> Vec<String> {
        service::save(&self.backend(), "p").unwrap();
        self.sim.snapshot()
    }

    /// Restores profile "p"; afterwards the log holds only what the restore did.
    fn restore(&self) -> Restored {
        self.sim.clear_log();
        service::restore(&self.backend(), "p").unwrap()
    }

    fn layout(&self) -> Layout {
        self.store.load("p").unwrap()
    }
}

fn entry(app: &str, title: &str) -> Entry {
    Entry {
        app: app.into(),
        title: title.into(),
    }
}

fn nothing() -> Vec<String> {
    Vec::new()
}

// ---- frames

#[test]
fn restoring_what_is_already_in_place_does_nothing() {
    let rig = Rig::new(two_screens());
    rig.sim.window("Notes", "list", "Main", 2, F);
    rig.sim.window("Notes", "hidden", "Main", 3, G);
    rig.sim.window("Mail", "inbox", "Side", 1, S);
    let before = rig.save();

    let restored = rig.restore();
    assert_eq!(
        restored,
        Restored {
            profile: "p".into(),
            moved: vec![],
            failed: vec![],
            notes: vec![],
        }
    );
    assert_eq!(rig.sim.actions(), nothing());
    assert_eq!(rig.sim.snapshot(), before);
    assert_eq!(rig.sim.looks.get(), 1);
    assert_eq!(rig.sim.waited_ms.get(), 0);
    assert_eq!(rig.sim.front.get(), FRONT_APP);
}

#[test]
fn a_window_that_was_moved_goes_back_with_one_look_at_the_desktop() {
    let rig = Rig::new(two_screens());
    let notes = rig.sim.window("Notes", "list", "Main", 2, F);
    rig.sim.window("Mail", "inbox", "Side", 1, S);
    let before = rig.save();
    rig.sim.user_moves(notes, "Main", 2, G);

    let restored = rig.restore();
    assert_eq!(restored.moved, [entry("Notes", "list")]);
    assert_eq!(restored.failed, []);
    assert_eq!(rig.sim.actions(), ["move Notes list"]);
    assert_eq!(rig.sim.snapshot(), before);
    assert_eq!(rig.sim.looks.get(), 1);
}

#[test]
fn a_window_on_a_desktop_that_is_not_in_view_is_moved_without_showing_it() {
    let rig = Rig::new(two_screens());
    let notes = rig.sim.window("Notes", "list", "Main", 3, F);
    let before = rig.save();
    rig.sim.user_moves(notes, "Main", 3, G);

    assert_eq!(rig.restore().moved, [entry("Notes", "list")]);
    assert_eq!(rig.sim.actions(), ["move Notes list"]);
    assert_eq!(rig.sim.snapshot(), before);
}

#[test]
fn moved_windows_are_reported_in_the_order_of_the_profile() {
    let rig = Rig::new(two_screens());
    let first = rig.sim.window("Notes", "one", "Main", 2, F);
    let second = rig.sim.window("Mail", "two", "Side", 1, S);
    rig.save();
    rig.sim
        .user_moves(second, "Side", 1, Rect::new(2200.0, 0.0, 500.0, 500.0));
    rig.sim.user_moves(first, "Main", 2, G);

    assert_eq!(
        rig.restore().moved,
        [entry("Notes", "one"), entry("Mail", "two")]
    );
}

#[test]
fn a_window_that_does_not_end_up_right_is_asked_again() {
    let rig = Rig::new(two_screens());
    let notes = rig.sim.window("Notes", "list", "Main", 2, F);
    let before = rig.save();
    rig.sim.user_moves(notes, "Main", 2, G);
    rig.sim.stubborn(notes, 1);

    assert_eq!(rig.restore().moved, [entry("Notes", "list")]);
    assert_eq!(rig.sim.actions(), ["move Notes list", "move Notes list"]);
    assert_eq!(rig.sim.snapshot(), before);
}

#[test]
fn a_window_that_never_ends_up_right_is_asked_three_times_and_not_called_a_failure() {
    let rig = Rig::new(two_screens());
    let notes = rig.sim.window("Notes", "list", "Main", 2, F);
    rig.save();
    rig.sim.user_moves(notes, "Main", 2, G);
    rig.sim.stubborn(notes, 9);

    let restored = rig.restore();
    assert_eq!(restored.moved, [entry("Notes", "list")]);
    assert_eq!(restored.failed, []);
    assert_eq!(rig.sim.count("move"), 3);
    // As close as its app allows: everything but the width.
    assert_eq!(
        rig.sim.frame_of(notes),
        Rect::new(100.0, 80.0, 640.0, 500.0)
    );
    assert_eq!(rig.sim.looks.get(), 1);
}

#[test]
fn a_window_that_refuses_to_move_is_reported_after_one_try() {
    let rig = Rig::new(two_screens());
    let notes = rig.sim.window("Notes", "list", "Main", 2, F);
    let other = rig.sim.window("Mail", "inbox", "Main", 2, G);
    rig.save();
    rig.sim.user_moves(notes, "Main", 2, G);
    rig.sim.user_moves(other, "Main", 2, F);
    rig.sim.refuses_to_move(notes);

    let restored = rig.restore();
    assert_eq!(restored.moved, [entry("Mail", "inbox")]);
    assert_eq!(restored.failed, [entry("Notes", "list")]);
    assert_eq!(rig.sim.actions(), ["move Notes list", "move Mail inbox"]);
}

// ---- desktops

#[test]
fn a_window_on_the_wrong_desktop_of_its_screen_hops_to_another_screen_and_back() {
    let rig = Rig::new(two_screens());
    let notes = rig.sim.window("Notes", "list", "Main", 3, F);
    let before = rig.save();
    rig.sim.user_moves(notes, "Main", 1, G);

    let restored = rig.restore();
    assert_eq!(restored.moved, [entry("Notes", "list")]);
    assert_eq!(restored.failed, []);
    assert_eq!(restored.notes, nothing());
    assert_eq!(
        rig.sim.actions(),
        [
            // Desktop 3 comes into view …
            "park 1000,600",
            "step Main right",
            // … the window leaves the screen and comes back, landing on it …
            "move Notes list",
            "move Notes list",
            // … and the screen shows what it showed when the profile was saved.
            "park 1000,600",
            "step Main left",
            "park 7,7",
            &format!("activate {FRONT_APP}"),
        ]
    );
    assert_eq!(rig.sim.snapshot(), before);
    assert_eq!(rig.sim.row("Main"), "1 [2] 3");
    assert_eq!(rig.sim.looks.get(), 2);
    assert_eq!(rig.sim.waited_ms.get(), 2 * (POLL_MS + STEP_SETTLE_MS));
    assert_eq!(rig.sim.front.get(), FRONT_APP);
}

#[test]
fn a_window_on_another_screen_lands_on_its_desktop_once_that_is_in_view() {
    let rig = Rig::new(two_screens());
    let notes = rig.sim.window("Notes", "list", "Main", 1, F);
    let before = rig.save();
    rig.sim.user_moves(notes, "Side", 1, S);

    assert_eq!(rig.restore().moved, [entry("Notes", "list")]);
    assert_eq!(
        rig.sim.actions(),
        [
            "park 1000,600",
            "step Main left",
            "move Notes list",
            "park 1000,600",
            "step Main right",
            "park 7,7",
            &format!("activate {FRONT_APP}"),
        ]
    );
    assert_eq!(rig.sim.snapshot(), before);
}

#[test]
fn a_window_whose_desktop_is_in_view_needs_no_stepping() {
    let rig = Rig::new(two_screens());
    let notes = rig.sim.window("Notes", "list", "Main", 2, F);
    let before = rig.save();
    rig.sim.user_moves(notes, "Main", 3, G);

    rig.restore();
    assert_eq!(rig.sim.actions(), ["move Notes list", "move Notes list"]);
    assert_eq!(rig.sim.snapshot(), before);
    assert_eq!(rig.sim.waited_ms.get(), 0);
}

#[test]
fn windows_for_several_desktops_are_taken_desktop_by_desktop() {
    let rig = Rig::new(two_screens());
    let a = rig.sim.window("Notes", "a", "Main", 3, F);
    let b = rig.sim.window("Notes", "b", "Main", 1, F);
    let c = rig.sim.window("Notes", "c", "Main", 3, G);
    let before = rig.save();
    for id in [a, b, c] {
        rig.sim
            .user_moves(id, "Main", 2, Rect::new(50.0, 50.0, 300.0, 300.0));
    }

    rig.restore();
    assert_eq!(rig.sim.snapshot(), before);
    // Desktop 1, then desktop 3 (two steps), then back to 2: four steps in all.
    assert_eq!(rig.sim.count("step"), 4);
}

#[test]
fn with_a_single_screen_a_window_changes_desktop_by_going_full_screen_and_back() {
    let rig = Rig::new(one_screen());
    let notes = rig.sim.window("Notes", "list", "Main", 3, F);
    let before = rig.save();
    rig.sim.user_moves(notes, "Main", 1, G);

    assert_eq!(rig.restore().moved, [entry("Notes", "list")]);
    assert_eq!(
        rig.sim.actions(),
        [
            "park 1000,600",
            "step Main right",
            &format!("fullscreen on {notes}"),
            &format!("fullscreen off {notes}"),
            "move Notes list",
            "park 1000,600",
            "step Main left",
            "park 7,7",
            &format!("activate {FRONT_APP}"),
        ]
    );
    assert_eq!(rig.sim.snapshot(), before);
    assert_eq!(rig.sim.row("Main"), "1 [2] 3");
}

#[test]
fn with_a_single_screen_an_app_without_full_screen_stays_on_its_desktop_and_is_reported() {
    let rig = Rig::new(one_screen());
    let notes = rig.sim.window("Notes", "list", "Main", 3, F);
    rig.save();
    rig.sim.user_moves(notes, "Main", 1, G);
    rig.sim.cannot_fullscreen(notes);

    let restored = rig.restore();
    assert_eq!(restored.moved, []);
    assert_eq!(restored.failed, [entry("Notes", "list")]);
    assert_eq!(rig.sim.count("fullscreen"), 1);
    // The frame is right all the same.
    assert_eq!(rig.sim.where_is(notes), "Main desktop 1");
    assert_eq!(rig.sim.frame_of(notes), F);
}

#[test]
fn a_desktop_that_no_longer_exists_means_the_last_one() {
    let rig = Rig::new(two_screens());
    let notes = rig.sim.window("Notes", "list", "Main", 3, F);
    rig.save();
    rig.sim.user_moves(notes, "Main", 1, F);
    rig.sim.user_closes_last_desktop("Main");

    assert_eq!(rig.restore().failed, []);
    assert_eq!(rig.sim.where_is(notes), "Main desktop 2");
    assert_eq!(rig.sim.row("Main"), "1 [2]");
}

#[test]
fn a_window_on_all_desktops_is_only_moved_between_screens() {
    let rig = Rig::new(two_screens());
    let notes = rig.sim.window("Notes", "list", "Main", 2, F);
    rig.sim.everywhere(notes);
    rig.save();
    assert_eq!(
        rig.layout().windows,
        [WindowInfo::new("Notes", "com.sim.notes", "list", F).on_display(MAIN)]
    );
    rig.sim.user_moves(notes, "Side", 1, S);

    assert_eq!(rig.restore().moved, [entry("Notes", "list")]);
    assert_eq!(rig.sim.actions(), ["move Notes list"]);
    assert_eq!(rig.sim.where_is(notes), "Main desktop 2");
    // It changed screens, so the result is checked.
    assert_eq!(rig.sim.looks.get(), 2);
}

// ---- full screen

#[test]
fn a_window_saved_as_full_screen_is_made_full_screen_again_on_its_screen() {
    let rig = Rig::new(two_screens());
    let notes = rig.sim.window("Notes", "list", "Side", 1, S);
    rig.sim.user_fullscreen(notes, true);
    let before = rig.save();
    assert_eq!(
        rig.layout().windows[0],
        WindowInfo::new(
            "Notes",
            "com.sim.notes",
            "list",
            Rect::new(2000.0, -400.0, 1000.0, 1600.0)
        )
        .in_full_screen(SIDE)
    );
    rig.sim.user_fullscreen(notes, false);
    rig.sim.user_moves(notes, "Main", 2, F);

    let restored = rig.restore();
    assert_eq!(restored.moved, [entry("Notes", "list")]);
    assert_eq!(restored.failed, []);
    assert_eq!(
        rig.sim.actions(),
        [
            "move Notes list",
            &format!("fullscreen on {notes}"),
            &format!("activate {FRONT_APP}"),
        ]
    );
    assert_eq!(rig.sim.snapshot(), before);
    assert_eq!(rig.sim.row("Side"), "1 [F:Notes list]");
    assert_eq!(rig.sim.looks.get(), 2);
    assert_eq!(rig.sim.waited_ms.get(), POLL_MS + FULLSCREEN_SETTLE_MS);
}

#[test]
fn a_window_already_on_its_screen_keeps_its_place_when_it_goes_full_screen() {
    let rig = Rig::new(two_screens());
    let notes = rig.sim.window("Notes", "list", "Side", 1, S);
    rig.sim.user_fullscreen(notes, true);
    rig.save();
    rig.sim.user_fullscreen(notes, false);

    rig.restore();
    assert_eq!(
        rig.sim.actions(),
        [
            format!("fullscreen on {notes}"),
            format!("activate {FRONT_APP}"),
        ]
    );
    // Out of full screen it is where it was before.
    rig.sim.user_fullscreen(notes, false);
    assert_eq!(rig.sim.frame_of(notes), S);
}

#[test]
fn a_window_saved_as_ordinary_comes_out_of_full_screen() {
    let rig = Rig::new(two_screens());
    let notes = rig.sim.window("Notes", "list", "Main", 2, F);
    let before = rig.save();
    rig.sim.user_fullscreen(notes, true);
    assert_eq!(rig.sim.row("Main"), "1 2 3 [F:Notes list]");

    assert_eq!(rig.restore().moved, [entry("Notes", "list")]);
    assert_eq!(
        rig.sim.actions(),
        [
            format!("fullscreen off {notes}"),
            format!("activate {FRONT_APP}"),
        ]
    );
    assert_eq!(rig.sim.snapshot(), before);
    assert_eq!(rig.sim.row("Main"), "1 [2] 3");
    assert_eq!(rig.sim.looks.get(), 2);
}

#[test]
fn a_window_that_leaves_a_full_screen_desktop_out_of_view_goes_back_where_it_came_from() {
    // Measured on a real Mac: the screen stays as it is, and the window returns to the
    // desktop it was on before — so nothing more has to be done.
    let rig = Rig::new(two_screens());
    let notes = rig.sim.window("Notes", "list", "Main", 1, F);
    let before = rig.save();
    rig.sim.user_fullscreen(notes, true);
    rig.sim.user_shows("Main", 2);
    assert_eq!(rig.sim.row("Main"), "1 [2] 3 F:Notes list");

    assert_eq!(rig.restore().moved, [entry("Notes", "list")]);
    assert_eq!(
        rig.sim.actions(),
        [
            format!("fullscreen off {notes}"),
            format!("activate {FRONT_APP}"),
        ]
    );
    assert_eq!(rig.sim.snapshot(), before);
    assert_eq!(rig.sim.row("Main"), "1 [2] 3");
}

#[test]
fn a_window_out_of_full_screen_is_then_put_on_its_desktop() {
    let rig = Rig::new(two_screens());
    let notes = rig.sim.window("Notes", "list", "Main", 3, F);
    let before = rig.save();
    rig.sim.user_moves(notes, "Main", 2, G);
    rig.sim.user_fullscreen(notes, true);

    assert_eq!(rig.restore().moved, [entry("Notes", "list")]);
    assert_eq!(rig.sim.snapshot(), before);
    assert_eq!(rig.sim.row("Main"), "1 [2] 3");
    // Out of full screen, onto its desktop, and a last look to see that all is well.
    assert_eq!(rig.sim.looks.get(), 3);
}

#[test]
fn full_screen_on_the_wrong_screen_comes_out_moves_over_and_goes_back_in() {
    let rig = Rig::new(two_screens());
    let notes = rig.sim.window("Notes", "list", "Side", 1, S);
    rig.sim.user_fullscreen(notes, true);
    let before = rig.save();
    rig.sim.user_fullscreen(notes, false);
    rig.sim.user_moves(notes, "Main", 2, F);
    rig.sim.user_fullscreen(notes, true);
    assert_eq!(rig.sim.where_is(notes), "Main full screen");

    let restored = rig.restore();
    assert_eq!(restored.moved, [entry("Notes", "list")]);
    assert_eq!(
        rig.sim.actions(),
        [
            format!("fullscreen off {notes}"),
            "move Notes list".to_string(),
            format!("fullscreen on {notes}"),
            format!("activate {FRONT_APP}"),
        ]
    );
    assert_eq!(rig.sim.snapshot(), before);
    assert_eq!(rig.sim.row("Main"), "1 [2] 3");
    assert_eq!(rig.sim.looks.get(), 3);
}

#[test]
fn full_screen_desktops_are_made_again_in_their_saved_order() {
    let rig = Rig::new(two_screens());
    // The app lists them the other way round.
    let second = rig.sim.window("Notes", "second", "Side", 1, S);
    let first = rig.sim.window("Notes", "first", "Side", 1, S);
    rig.sim.user_fullscreen(first, true);
    rig.sim.user_fullscreen(second, true);
    rig.sim.user_shows("Side", 2);
    assert_eq!(rig.sim.row("Side"), "1 [F:Notes first] F:Notes second");
    let before = rig.save();
    rig.sim.user_fullscreen(first, false);
    rig.sim.user_fullscreen(second, false);

    rig.restore();
    assert_eq!(rig.sim.snapshot(), before);
    assert_eq!(rig.sim.row("Side"), "1 [F:Notes first] F:Notes second");
    assert_eq!(
        rig.sim.actions(),
        [
            format!("fullscreen on {first}"),
            format!("fullscreen on {second}"),
            // Side is left on the newest full-screen desktop; it showed the other one.
            "park 2500,400".to_string(),
            "step Side left".to_string(),
            "park 7,7".to_string(),
            format!("activate {FRONT_APP}"),
        ]
    );
}

#[test]
fn frames_are_set_before_windows_go_full_screen() {
    let rig = Rig::new(two_screens());
    let plain = rig.sim.window("Mail", "inbox", "Main", 2, F);
    let full = rig.sim.window("Notes", "list", "Side", 1, S);
    rig.sim.user_fullscreen(full, true);
    let before = rig.save();
    rig.sim.user_fullscreen(full, false);
    rig.sim.user_moves(plain, "Main", 2, G);

    let restored = rig.restore();
    assert_eq!(
        restored.moved,
        [entry("Mail", "inbox"), entry("Notes", "list")]
    );
    assert_eq!(
        rig.sim.actions(),
        [
            "move Mail inbox".to_string(),
            format!("fullscreen on {full}"),
            format!("activate {FRONT_APP}"),
        ]
    );
    assert_eq!(rig.sim.snapshot(), before);
}

#[test]
fn an_app_that_cannot_go_full_screen_is_reported_without_waiting() {
    let rig = Rig::new(two_screens());
    let notes = rig.sim.window("Notes", "list", "Side", 1, S);
    rig.sim.user_fullscreen(notes, true);
    rig.save();
    rig.sim.user_fullscreen(notes, false);
    rig.sim.cannot_fullscreen(notes);

    let restored = rig.restore();
    assert_eq!(restored.moved, []);
    assert_eq!(restored.failed, [entry("Notes", "list")]);
    assert_eq!(rig.sim.actions(), [format!("fullscreen on {notes}")]);
    assert_eq!(rig.sim.waited_ms.get(), 0);
}

#[test]
fn a_full_screen_change_is_waited_for() {
    let rig = Rig::new(two_screens());
    let notes = rig.sim.window("Notes", "list", "Side", 1, S);
    rig.sim.user_fullscreen(notes, true);
    let before = rig.save();
    rig.sim.user_fullscreen(notes, false);
    rig.sim.fullscreen_lag.set(4);

    assert_eq!(rig.restore().failed, []);
    assert_eq!(rig.sim.snapshot(), before);
    assert_eq!(rig.sim.waited_ms.get(), 4 * POLL_MS + FULLSCREEN_SETTLE_MS);
}

#[test]
fn a_screen_is_given_time_to_arrive_at_a_new_full_screen_desktop() {
    // macOS makes the full-screen desktop first and shows it when its animation is over.
    // Going on before that would leave the screen on the full-screen desktop in the end.
    let rig = Rig::new(two_screens());
    let notes = rig.sim.window("Notes", "list", "Side", 1, S);
    rig.sim.user_fullscreen(notes, true);
    rig.sim.user_shows("Side", 1);
    let before = rig.save();
    rig.sim.user_fullscreen(notes, false);
    rig.sim.switch_lag.set(6);

    assert_eq!(rig.restore().failed, []);
    let waited = rig.sim.waited_ms.get();
    rig.sim.time_passes();
    assert_eq!(rig.sim.snapshot(), before);
    assert_eq!(rig.sim.row("Side"), "[1] F:Notes list");
    // Six polls for the screen to arrive, then one step back to the desktop it showed.
    assert_eq!(
        waited,
        6 * POLL_MS + FULLSCREEN_SETTLE_MS + POLL_MS + STEP_SETTLE_MS
    );
}

#[test]
fn leaving_full_screen_is_over_when_its_desktop_is_gone() {
    let rig = Rig::new(two_screens());
    let notes = rig.sim.window("Notes", "list", "Main", 2, F);
    let other = rig.sim.window("Mail", "inbox", "Side", 1, S);
    let before = rig.save();
    rig.sim.user_fullscreen(notes, true);
    rig.sim.user_fullscreen(other, true);
    rig.sim.fullscreen_lag.set(3);

    assert_eq!(rig.restore().failed, []);
    assert_eq!(rig.sim.snapshot(), before);
    // Each of the two is waited for in turn.
    assert_eq!(
        rig.sim.waited_ms.get(),
        2 * (3 * POLL_MS + FULLSCREEN_SETTLE_MS)
    );
}

#[test]
fn a_full_screen_change_that_never_comes_is_given_up_on() {
    let rig = Rig::new(two_screens());
    let notes = rig.sim.window("Notes", "list", "Side", 1, S);
    rig.sim.user_fullscreen(notes, true);
    rig.save();
    rig.sim.user_fullscreen(notes, false);
    rig.sim.fullscreen_lag.set(1000);

    let restored = rig.restore();
    assert_eq!(restored.failed, [entry("Notes", "list")]);
    assert_eq!(rig.sim.count("fullscreen"), 1);
    assert_eq!(
        rig.sim.waited_ms.get(),
        POLLS as u64 * POLL_MS + FULLSCREEN_SETTLE_MS
    );
}

// ---- windows that cannot be reached

#[test]
fn a_window_its_app_never_handed_out_is_reached_by_showing_its_desktop_once() {
    let rig = Rig::new(two_screens());
    let notes = rig.sim.window("Notes", "list", "Main", 3, F);
    rig.sim.never_listed(notes);
    let before = rig.save();
    // Saved from what the window server knows: no title.
    assert_eq!(
        rig.layout().windows,
        [WindowInfo::new("Notes", "com.sim.notes", "", F).on_desktop(MAIN, 3)]
    );
    rig.sim.user_moves(notes, "Main", 3, G);

    let restored = rig.restore();
    assert_eq!(restored.moved, [entry("Notes", "list")]);
    assert_eq!(restored.failed, []);
    assert_eq!(
        rig.sim.actions(),
        [
            "park 1000,600",
            "step Main right",
            "move Notes list",
            "park 1000,600",
            "step Main left",
            "park 7,7",
            &format!("activate {FRONT_APP}"),
        ]
    );
    assert_eq!(rig.sim.snapshot(), before);
    // A look, the question that makes the app hand it out, and a second look.
    assert_eq!(rig.sim.looks.get(), 3);
}

#[test]
fn a_window_that_cannot_be_reached_but_is_in_place_is_left_alone() {
    let rig = Rig::new(two_screens());
    let notes = rig.sim.window("Notes", "list", "Main", 3, F);
    rig.sim.never_listed(notes);
    rig.save();

    let restored = rig.restore();
    assert_eq!((restored.moved, restored.failed), (vec![], vec![]));
    assert_eq!(rig.sim.actions(), nothing());
}

#[test]
fn several_unreachable_windows_on_one_desktop_cost_one_visit() {
    let rig = Rig::new(two_screens());
    let a = rig.sim.window("Notes", "a", "Main", 3, F);
    let b = rig.sim.window("Notes", "b", "Main", 3, G);
    rig.sim.never_listed(a);
    rig.sim.never_listed(b);
    rig.save();
    rig.sim
        .user_moves(a, "Main", 3, Rect::new(10.0, 10.0, 300.0, 300.0));
    rig.sim
        .user_moves(b, "Main", 3, Rect::new(20.0, 20.0, 300.0, 300.0));

    assert_eq!(rig.restore().failed, []);
    assert_eq!(rig.sim.frame_of(a), F);
    assert_eq!(rig.sim.frame_of(b), G);
    assert_eq!(rig.sim.count("step"), 2);
}

#[test]
fn a_window_that_stays_out_of_reach_is_reported_and_not_tried_again() {
    let rig = Rig::new(two_screens());
    let notes = rig.sim.window("Notes", "list", "Main", 1, F);
    rig.sim.withheld(notes);
    rig.save();
    rig.sim.user_moves(notes, "Main", 1, G);

    let restored = rig.restore();
    assert_eq!(restored.moved, []);
    assert_eq!(restored.failed, [entry("Notes", "")]);
    assert_eq!(
        rig.sim.actions(),
        [
            "park 1000,600",
            "step Main left",
            "park 1000,600",
            "step Main right",
            "park 7,7",
            &format!("activate {FRONT_APP}"),
        ]
    );
}

// ---- when desktops cannot be switched

#[test]
fn with_the_shortcuts_switched_off_no_key_is_sent_and_the_user_is_told() {
    let rig = Rig::new(two_screens());
    let notes = rig.sim.window("Notes", "list", "Main", 3, F);
    let other = rig.sim.window("Mail", "inbox", "Main", 1, F);
    rig.save();
    rig.sim.user_moves(notes, "Main", 1, G);
    rig.sim.user_moves(other, "Main", 3, G);
    rig.sim.user_shows("Main", 1);
    rig.sim.shortcuts_on.set(false);

    let restored = rig.restore();
    // Desktop 1 happens to be in view, so Mail can go there; desktop 3 cannot be shown.
    assert_eq!(restored.moved, [entry("Mail", "inbox")]);
    assert_eq!(restored.failed, [entry("Notes", "list")]);
    assert_eq!(restored.notes, [NO_STEPPING_NOTE]);
    assert_eq!(
        rig.sim.actions(),
        ["move Mail inbox", "move Mail inbox", "move Notes list"]
    );
    assert_eq!(rig.sim.where_is(other), "Main desktop 1");
    // Notes gets its frame, on the desktop it is on.
    assert_eq!(rig.sim.where_is(notes), "Main desktop 1");
    assert_eq!(rig.sim.frame_of(notes), F);
    // Main showed desktop 2 when the profile was saved; that cannot be put back either.
    assert_eq!(rig.sim.row("Main"), "[1] 2 3");
    assert_eq!(rig.sim.waited_ms.get(), 0);
    assert_eq!(rig.sim.front.get(), FRONT_APP);
}

#[test]
fn with_the_shortcuts_switched_off_a_window_out_of_reach_stays_out_of_reach() {
    let rig = Rig::new(two_screens());
    let notes = rig.sim.window("Notes", "list", "Main", 3, F);
    rig.sim.never_listed(notes);
    rig.save();
    rig.sim.user_moves(notes, "Main", 3, G);
    rig.sim.shortcuts_on.set(false);

    let restored = rig.restore();
    assert_eq!(restored.moved, []);
    assert_eq!(restored.failed, [entry("Notes", "")]);
    assert_eq!(restored.notes, [NO_STEPPING_NOTE]);
    assert_eq!(rig.sim.actions(), nothing());
    // Two looks, and no asking the apps in between: its desktop never came into view.
    assert_eq!(rig.sim.looks.get(), 2);
}

#[test]
fn when_macos_ignores_the_shortcut_it_is_pressed_once_and_the_user_is_told() {
    let rig = Rig::new(two_screens());
    let notes = rig.sim.window("Notes", "list", "Main", 3, F);
    rig.save();
    rig.sim.user_moves(notes, "Main", 1, G);
    rig.sim.user_shows("Main", 1);
    rig.sim.keys_work.set(false);

    let restored = rig.restore();
    assert_eq!(restored.failed, [entry("Notes", "list")]);
    assert_eq!(restored.notes, [NO_STEPPING_NOTE]);
    assert_eq!(
        rig.sim.actions(),
        [
            "park 1000,600",
            "step Main right",
            "move Notes list",
            "park 7,7",
            &format!("activate {FRONT_APP}"),
        ]
    );
    assert_eq!(rig.sim.waited_ms.get(), POLLS as u64 * POLL_MS);
}

// ---- what the screens show afterwards

#[test]
fn each_screen_ends_up_showing_what_it_showed_when_the_profile_was_saved() {
    let rig = Rig::new(two_screens());
    rig.sim.window("Notes", "list", "Main", 2, F);
    rig.save();
    assert_eq!(
        rig.layout().desktops,
        [
            SavedScreen {
                display: MAIN.into(),
                desktops: 3,
                showing: Showing::Desktop(2),
            },
            SavedScreen {
                display: SIDE.into(),
                desktops: 1,
                showing: Showing::Desktop(1),
            },
        ]
    );
    rig.sim.user_shows("Main", 1);

    let restored = rig.restore();
    assert_eq!((restored.moved, restored.failed), (vec![], vec![]));
    assert_eq!(
        rig.sim.actions(),
        [
            "park 1000,600",
            "step Main right",
            "park 7,7",
            &format!("activate {FRONT_APP}"),
        ]
    );
    assert_eq!(rig.sim.row("Main"), "1 [2] 3");
    assert_eq!(rig.sim.waited_ms.get(), POLL_MS + STEP_SETTLE_MS);
}

#[test]
fn a_screen_that_showed_a_full_screen_app_shows_it_again() {
    let rig = Rig::new(two_screens());
    let other = rig.sim.window("Mail", "inbox", "Side", 1, S);
    let notes = rig.sim.window("Notes", "list", "Side", 1, S);
    rig.sim.window("Notes", "plain", "Side", 1, S);
    rig.sim.user_fullscreen(other, true);
    rig.sim.user_fullscreen(notes, true);
    rig.save();
    assert_eq!(
        rig.layout().desktops[1].showing,
        Showing::FullScreen {
            bundle_id: "com.sim.notes".into(),
            title: "list".into(),
        }
    );
    // Its title has changed since; it is still the app's full-screen window there.
    rig.sim.retitle(notes, "another list");
    rig.sim.user_shows("Side", 1);
    rig.sim.pointer_at(2400.0, 0.0);

    rig.restore();
    assert_eq!(rig.sim.row("Side"), "1 F:Mail inbox [F:Notes another list]");
    assert_eq!(rig.sim.count("step"), 2);
}

#[test]
fn of_an_apps_full_screen_windows_the_one_with_the_saved_title_is_shown() {
    let sim = Sim::new()
        .screen("Main", 10, Rect::new(0.0, 0.0, 2000.0, 1200.0), 1, 1)
        .screen("Side", 20, Rect::new(2000.0, -400.0, 1000.0, 1600.0), 1, 1);
    let rig = Rig::new(sim);
    let on_main = rig.sim.window("Notes", "list", "Main", 1, F);
    let other = rig.sim.window("Notes", "other", "Side", 1, S);
    let wanted = rig.sim.window("Notes", "list", "Side", 1, S);
    rig.sim.user_fullscreen(on_main, true);
    rig.sim.user_fullscreen(other, true);
    rig.sim.user_fullscreen(wanted, true);
    rig.save();
    rig.sim.user_shows("Side", 1);

    rig.restore();
    assert_eq!(rig.sim.row("Side"), "1 F:Notes other [F:Notes list]");
    assert_eq!(rig.sim.row("Main"), "1 [F:Notes list]");
}

#[test]
fn a_screen_with_fewer_desktops_than_before_shows_its_last_one() {
    let rig = Rig::new(two_screens());
    rig.sim.window("Notes", "list", "Main", 2, F);
    rig.sim.user_shows("Main", 3);
    rig.save();
    rig.sim.user_shows("Main", 1);
    rig.sim.user_closes_last_desktop("Main");

    rig.restore();
    assert_eq!(rig.sim.row("Main"), "1 [2]");
}

#[test]
fn what_a_screen_showed_is_left_alone_when_it_is_unknown_or_the_screen_is_gone() {
    let rig = Rig::new(two_screens());
    rig.sim.window("Notes", "list", "Main", 2, F);
    rig.save();
    let mut layout = rig.layout();
    layout.desktops[0].showing = Showing::Unknown;
    layout.desktops[1].display = "9-9-9".into();
    layout.desktops[1].showing = Showing::Desktop(1);
    layout.desktops.push(SavedScreen {
        display: SIDE.into(),
        desktops: 1,
        showing: Showing::FullScreen {
            bundle_id: "com.sim.notes".into(),
            title: "list".into(),
        },
    });
    rig.store.save(&layout).unwrap();
    rig.sim.user_shows("Main", 1);

    rig.restore();
    assert_eq!(rig.sim.actions(), nothing());
    assert_eq!(rig.sim.row("Main"), "[1] 2 3");
}

// ---- profiles and Macs that know nothing about desktops

#[test]
fn a_profile_from_before_desktops_were_recorded_only_touches_what_is_in_view() {
    let rig = Rig::new(two_screens());
    let hidden = rig.sim.window("Notes", "list", "Main", 3, G);
    let in_view = rig.sim.window("Notes", "other", "Main", 2, G);
    let full = rig.sim.window("Notes", "full", "Side", 1, S);
    rig.sim.user_fullscreen(full, true);
    let old = Layout {
        profile: "p".into(),
        saved_at: "2026-11-11T09:42:04Z".into(),
        displays: vec![],
        windows: vec![WindowInfo::new("Notes", "com.sim.notes", "list", F)],
        desktops: vec![],
    };
    rig.store.save(&old).unwrap();
    rig.sim.user_shows("Side", 1);

    let restored = rig.restore();
    assert_eq!(restored.moved, [entry("Notes", "other")]);
    assert_eq!(rig.sim.actions(), ["move Notes other"]);
    assert_eq!(rig.sim.frame_of(in_view), F);
    assert_eq!(rig.sim.frame_of(hidden), G);
    assert_eq!(rig.sim.where_is(full), "Side full screen");
    assert_eq!(rig.sim.row("Side"), "[1] F:Notes full");
}

#[test]
fn where_macos_tells_nothing_about_desktops_a_new_profile_restores_what_is_in_view() {
    let (_dir, store) = temp_store();
    let sys = FakeSystem::new(
        vec![display("Built-in", 10, 0.0)],
        vec![window("Notes", "list", 5.0), window("Notes", "other", 6.0)],
    );
    let backend = Backend {
        sys: &sys,
        store: &store,
        clock: &CLOCK,
    };
    let layout = Layout {
        profile: "p".into(),
        saved_at: "2026-11-11T09:42:04Z".into(),
        displays: vec![],
        windows: vec![
            WindowInfo::new("Notes", "com.test.notes", "was full", F).in_full_screen(MAIN),
            WindowInfo::new("Notes", "com.test.notes", "list", F).on_desktop(MAIN, 2),
        ],
        desktops: vec![SavedScreen {
            display: MAIN.into(),
            desktops: 2,
            showing: Showing::Desktop(2),
        }],
    };
    let report = restore(&backend, &layout).unwrap();
    assert_eq!(report.moved, [entry("Notes", "list")]);
    assert_eq!(report.failed, []);
    assert_eq!(sys.moves(), [(0, F)]);
}

#[test]
fn restoring_needs_accessibility_permission() {
    let rig = Rig::new(two_screens());
    rig.sim.window("Notes", "list", "Main", 2, F);
    rig.save();
    rig.sim.trusted.set(false);
    assert_eq!(
        restore(&rig.backend(), &rig.layout()),
        Err(Error::NotTrusted)
    );
    assert_eq!(
        service::restore(&rig.backend(), "p"),
        Err(Error::NotTrusted)
    );
}

#[test]
fn types_are_cloneable_and_printable() {
    let line = entry("Notes", "list");
    assert_eq!(line.clone(), line);
    assert!(format!("{line:?}").starts_with("Entry"));
    let report = Report::default();
    assert_eq!(report.clone(), report);
    assert!(format!("{report:?}").starts_with("Report"));
}
