use super::*;
use crate::desktops::{DesktopMap, Kind, RawScreen, RawSpace};
use crate::displays::RawDisplay;
use crate::geometry::Rect;

fn raw() -> RawWindow {
    RawWindow {
        app: Some("Ghostty".into()),
        bundle_id: Some("com.mitchellh.ghostty".into()),
        pid: 7,
        handle: 0,
        window_id: Some(100),
        subrole: Some(STANDARD_WINDOW.into()),
        minimized: Some(false),
        fullscreen: Some(false),
        title: Some("zsh".into()),
        position: Some((10.0, 20.0)),
        size: Some((300.0, 200.0)),
        spaces: vec![],
    }
}

/// No desktop information at all (what an unsupported macOS would give).
fn unknown() -> DesktopMap {
    DesktopMap::default()
}

/// One screen showing desktop 2 of: desktop 1 (id 11), desktop 2 (id 12), a full-screen app (id 13).
fn desktops() -> DesktopMap {
    desktops_showing(12)
}

/// The same screen, showing the desktop with this id.
fn desktops_showing(current: u64) -> DesktopMap {
    let display = RawDisplay {
        name: Some("Main".into()),
        vendor: 1,
        model: 2,
        serial: 3,
        uuid: Some("U".into()),
        bounds: Rect::new(0.0, 0.0, 1000.0, 800.0),
    };
    let spaces = [(11, false), (12, false), (13, true)]
        .map(|(id, fullscreen)| RawSpace { id, fullscreen })
        .to_vec();
    DesktopMap::new(
        &[display],
        &[RawScreen {
            uuid: "U".into(),
            current,
            spaces,
        }],
    )
}

fn on(spaces: &[u64], id: u32) -> RawWindow {
    RawWindow {
        window_id: Some(id),
        spaces: spaces.to_vec(),
        ..raw()
    }
}

fn server(id: u32, spaces: &[u64]) -> ServerWindow {
    ServerWindow {
        app: Some("Notes".into()),
        bundle_id: Some("com.apple.Notes".into()),
        pid: 9,
        window_id: id,
        layer: 0,
        alpha: 1.0,
        bounds: Rect::new(5.0, 6.0, 700.0, 500.0),
        spaces: spaces.to_vec(),
    }
}

#[test]
fn standard_window_becomes_live() {
    assert_eq!(
        live_windows(&[raw()], &unknown()),
        vec![LiveWindow {
            handle: 0,
            window_id: Some(100),
            app: "Ghostty".into(),
            bundle_id: "com.mitchellh.ghostty".into(),
            title: "zsh".into(),
            frame: Rect::new(10.0, 20.0, 300.0, 200.0),
            fullscreen: false,
            place: None,
        }]
    );
}

#[test]
fn the_handle_given_by_the_macos_layer_is_kept() {
    let dialog = RawWindow {
        subrole: Some("AXDialog".into()),
        handle: 4,
        ..raw()
    };
    let live = live_windows(
        &[
            dialog,
            RawWindow { handle: 7, ..raw() },
            RawWindow { handle: 2, ..raw() },
        ],
        &unknown(),
    );
    assert_eq!(live.iter().map(|w| w.handle).collect::<Vec<_>>(), [7, 2]);
}

#[test]
fn skips_windows_that_are_not_standard() {
    let none = RawWindow {
        subrole: None,
        ..raw()
    };
    let floating = RawWindow {
        subrole: Some("AXFloatingWindow".into()),
        ..raw()
    };
    assert!(live_windows(&[none, floating], &unknown()).is_empty());
}

#[test]
fn skips_minimised_windows() {
    let minimized = RawWindow {
        minimized: Some(true),
        ..raw()
    };
    assert!(live_windows(&[minimized], &unknown()).is_empty());
}

#[test]
fn full_screen_windows_are_kept_and_flagged() {
    let fullscreen = RawWindow {
        fullscreen: Some(true),
        ..raw()
    };
    let live = live_windows(&[fullscreen, raw()], &unknown());
    assert_eq!(live.len(), 2);
    assert!(live[0].fullscreen);
    assert!(!live[1].fullscreen);
}

#[test]
fn a_window_on_a_full_screen_desktop_counts_as_full_screen_even_if_the_app_does_not_say_so() {
    let silent = RawWindow {
        fullscreen: None,
        ..on(&[13], 1)
    };
    assert!(live_windows(&[silent], &desktops())[0].fullscreen);
    assert!(!live_windows(&[on(&[12], 1)], &desktops())[0].fullscreen);
}

#[test]
fn each_window_gets_its_place() {
    let live = live_windows(&[on(&[12], 1), on(&[11], 2), on(&[13], 3)], &desktops());
    let places: Vec<(Kind, bool)> = live
        .iter()
        .map(|w| w.place.as_ref().map(|p| (p.kind, p.showing)).unwrap())
        .collect();
    assert_eq!(
        places,
        [
            (Kind::Desktop(2), true),
            (Kind::Desktop(1), false),
            (Kind::FullScreen, false),
        ]
    );
    assert_eq!(live[1].window_id, Some(2));
}

#[test]
fn a_window_on_no_desktop_is_a_background_tab_and_is_left_out() {
    // macOS gives every tab its own window; only the selected tab is on a desktop.
    let live = live_windows(&[on(&[12], 1), on(&[], 2), on(&[99], 3)], &desktops());
    assert_eq!(
        live.iter().map(|w| w.window_id).collect::<Vec<_>>(),
        [Some(1)]
    );
    // Without desktop information there is no telling, so everything is kept.
    assert_eq!(
        live_windows(&[on(&[12], 1), on(&[], 2)], &unknown()).len(),
        2
    );
}

#[test]
fn where_desktops_are_known_only_a_window_on_a_full_screen_desktop_is_full_screen() {
    // An app says "full screen" the moment it is asked to be, before macOS has made the
    // full-screen desktop — and keeps saying it for a while if macOS never does.
    let claims = RawWindow {
        fullscreen: Some(true),
        ..on(&[12], 1)
    };
    assert!(!live_windows(&[claims], &desktops())[0].fullscreen);
    let denies = RawWindow {
        fullscreen: Some(false),
        ..on(&[13], 1)
    };
    assert!(live_windows(&[denies], &desktops())[0].fullscreen);
}

#[test]
fn plain_windows_in_view_are_the_ones_version_0_1_handled() {
    let live = live_windows(&[on(&[12], 1), on(&[11], 2)], &desktops());
    let plain: Vec<bool> = live.iter().map(LiveWindow::is_plain_in_view).collect();
    // In view; on a hidden desktop.
    assert_eq!(plain, [true, false]);
    // Full screen, although in view.
    let full = &live_windows(&[on(&[13], 3)], &desktops_showing(13))[0];
    assert!(full.place.as_ref().unwrap().showing);
    assert!(!full.is_plain_in_view());
    // Place unknown (no desktop information): treated as in view.
    assert!(live_windows(&[on(&[], 4)], &unknown())[0].is_plain_in_view());
    let claims_full_screen = RawWindow {
        fullscreen: Some(true),
        ..on(&[], 5)
    };
    assert!(!live_windows(&[claims_full_screen], &unknown())[0].is_plain_in_view());
}

#[test]
fn windows_only_the_window_server_knows_are_reported_as_unreached() {
    let found = unreached(
        &[on(&[12], 1)],
        &[server(1, &[12]), server(2, &[11])],
        &desktops(),
    );
    assert_eq!(found.len(), 1);
    assert_eq!(
        found[0],
        Unreached {
            window_id: 2,
            pid: 9,
            app: "Notes".into(),
            bundle_id: "com.apple.Notes".into(),
            frame: Rect::new(5.0, 6.0, 700.0, 500.0),
            place: desktops().place(&[11]).unwrap(),
        }
    );
}

#[test]
fn a_window_accessibility_knows_is_never_unreached_whatever_kind_it_is() {
    let dialog = RawWindow {
        subrole: Some("AXDialog".into()),
        ..on(&[11], 2)
    };
    assert!(unreached(&[dialog], &[server(2, &[11])], &desktops()).is_empty());
}

#[test]
fn helper_windows_are_not_mistaken_for_real_ones() {
    let map = desktops();
    let odd = |change: fn(&mut ServerWindow)| {
        let mut window = server(2, &[11]);
        change(&mut window);
        unreached(&[], &[window], &map).len()
    };
    assert_eq!(odd(|_| {}), 1);
    assert_eq!(odd(|w| w.layer = 3), 0);
    assert_eq!(odd(|w| w.layer = -1), 0);
    assert_eq!(odd(|w| w.alpha = 0.0), 0);
    assert_eq!(odd(|w| w.spaces = vec![]), 0);
    assert_eq!(odd(|w| w.spaces = vec![99]), 0);
    assert_eq!(odd(|w| w.bounds.width = MIN_WIDTH), 1);
    assert_eq!(odd(|w| w.bounds.width = MIN_WIDTH - 1.0), 0);
    assert_eq!(odd(|w| w.bounds.height = MIN_HEIGHT), 1);
    assert_eq!(odd(|w| w.bounds.height = MIN_HEIGHT - 1.0), 0);
}

#[test]
fn on_a_full_screen_desktop_only_the_one_real_window_counts() {
    // macOS adds a title-bar strip window to a full-screen desktop; only the window
    // server lists it.
    let strip = |id: u32| {
        let mut window = server(id, &[13]);
        window.bounds = Rect::new(0.0, 0.0, 1000.0, 68.0);
        window
    };
    let big = |id: u32| {
        let mut window = server(id, &[13]);
        window.bounds = Rect::new(0.0, 0.0, 1000.0, 800.0);
        window
    };
    // The app's own window is known: nothing on that desktop is unreached.
    let known = on(&[13], 1);
    assert!(unreached(&[known], &[big(1), strip(2)], &desktops()).is_empty());
    // Nothing known there: the largest is the window, the strip is ignored.
    let found = unreached(&[], &[strip(2), big(3), strip(4)], &desktops());
    assert_eq!(found.iter().map(|w| w.window_id).collect::<Vec<_>>(), [3]);
    let found = unreached(&[], &[big(3), strip(2)], &desktops());
    assert_eq!(found.iter().map(|w| w.window_id).collect::<Vec<_>>(), [3]);
    // "Largest" means area: a wide strip is smaller than a modest square.
    let square = |id: u32| {
        let mut window = server(id, &[13]);
        window.bounds = Rect::new(0.0, 0.0, 300.0, 300.0);
        window
    };
    let found = unreached(&[], &[strip(2), square(6)], &desktops());
    assert_eq!(found.iter().map(|w| w.window_id).collect::<Vec<_>>(), [6]);
    // Two of the same size: the first listed is taken, never both.
    let found = unreached(&[], &[big(3), big(5)], &desktops());
    assert_eq!(found.iter().map(|w| w.window_id).collect::<Vec<_>>(), [3]);
    // A known window on another desktop does not hide anything here.
    let elsewhere = on(&[12], 9);
    assert_eq!(unreached(&[elsewhere], &[big(3)], &desktops()).len(), 1);
    // A known dialog on the full-screen desktop is not its window.
    let dialog = RawWindow {
        subrole: Some("AXDialog".into()),
        ..on(&[13], 7)
    };
    assert_eq!(unreached(&[dialog], &[big(3)], &desktops()).len(), 1);
    // Ordinary desktops may hold any number of unreached windows.
    assert_eq!(
        unreached(&[], &[server(5, &[11]), server(6, &[11])], &desktops()).len(),
        2
    );
}

#[test]
fn nothing_is_unreached_when_desktops_are_unknown() {
    assert!(unreached(&[], &[server(2, &[11])], &unknown()).is_empty());
}

#[test]
fn unreached_windows_fall_back_on_names_like_live_ones() {
    let anonymous = ServerWindow {
        app: None,
        bundle_id: None,
        ..server(2, &[11])
    };
    let found = unreached(&[], &[anonymous], &desktops());
    assert_eq!(found[0].app, "pid 9");
    assert_eq!(found[0].bundle_id, "pid 9");
    let no_bundle = ServerWindow {
        bundle_id: None,
        ..server(2, &[11])
    };
    assert_eq!(
        unreached(&[], &[no_bundle], &desktops())[0].bundle_id,
        "Notes"
    );
}

#[test]
fn unknown_minimized_or_fullscreen_state_is_kept() {
    let undecided = RawWindow {
        minimized: None,
        fullscreen: None,
        ..raw()
    };
    let live = live_windows(&[undecided], &unknown());
    assert_eq!(live.len(), 1);
    assert!(!live[0].fullscreen);
}

#[test]
fn skips_windows_without_position_or_size() {
    let no_position = RawWindow {
        position: None,
        ..raw()
    };
    let no_size = RawWindow {
        size: None,
        ..raw()
    };
    assert!(live_windows(&[no_position, no_size], &unknown()).is_empty());
}

#[test]
fn falls_back_when_names_are_missing() {
    let no_bundle = RawWindow {
        bundle_id: None,
        ..raw()
    };
    assert_eq!(
        live_windows(&[no_bundle], &unknown())[0].bundle_id,
        "Ghostty"
    );

    let anonymous = RawWindow {
        app: None,
        bundle_id: None,
        title: None,
        ..raw()
    };
    let live = live_windows(&[anonymous], &unknown());
    assert_eq!(live[0].app, "pid 7");
    assert_eq!(live[0].bundle_id, "pid 7");
    assert_eq!(live[0].title, "");

    let no_app = RawWindow { app: None, ..raw() };
    let live = live_windows(&[no_app], &unknown());
    assert_eq!(live[0].app, "pid 7");
    assert_eq!(live[0].bundle_id, "com.mitchellh.ghostty");
}

#[test]
fn types_are_cloneable_and_printable() {
    let r = raw();
    assert_eq!(r.clone(), r);
    assert!(format!("{r:?}").starts_with("RawWindow"));
    let live = live_windows(&[r], &unknown());
    assert_eq!(live.clone(), live);
    assert!(format!("{:?}", live[0]).starts_with("LiveWindow"));
    let s = server(1, &[11]);
    assert_eq!(s.clone(), s);
    assert!(format!("{s:?}").starts_with("ServerWindow"));
    let u = unreached(&[], &[s], &desktops());
    assert_eq!(u.clone(), u);
    assert!(format!("{:?}", u[0]).starts_with("Unreached"));
}
