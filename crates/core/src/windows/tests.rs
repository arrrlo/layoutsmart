use super::*;
use crate::geometry::Rect;

fn raw() -> RawWindow {
    RawWindow {
        app: Some("Ghostty".into()),
        bundle_id: Some("com.mitchellh.ghostty".into()),
        pid: 7,
        subrole: Some(STANDARD_WINDOW.into()),
        minimized: Some(false),
        fullscreen: Some(false),
        title: Some("zsh".into()),
        position: Some((10.0, 20.0)),
        size: Some((300.0, 200.0)),
    }
}

#[test]
fn standard_window_becomes_live() {
    assert_eq!(
        live_windows(&[raw()]),
        vec![LiveWindow {
            handle: 0,
            app: "Ghostty".into(),
            bundle_id: "com.mitchellh.ghostty".into(),
            title: "zsh".into(),
            frame: Rect::new(10.0, 20.0, 300.0, 200.0),
        }]
    );
}

#[test]
fn handle_is_the_index_in_the_raw_list() {
    let dialog = RawWindow {
        subrole: Some("AXDialog".into()),
        ..raw()
    };
    let live = live_windows(&[dialog, raw(), raw()]);
    assert_eq!(live.iter().map(|w| w.handle).collect::<Vec<_>>(), [1, 2]);
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
    assert!(live_windows(&[none, floating]).is_empty());
}

#[test]
fn skips_minimized_and_fullscreen() {
    let minimized = RawWindow {
        minimized: Some(true),
        ..raw()
    };
    let fullscreen = RawWindow {
        fullscreen: Some(true),
        ..raw()
    };
    assert!(live_windows(&[minimized, fullscreen]).is_empty());
}

#[test]
fn unknown_minimized_or_fullscreen_state_is_kept() {
    let unknown = RawWindow {
        minimized: None,
        fullscreen: None,
        ..raw()
    };
    assert_eq!(live_windows(&[unknown]).len(), 1);
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
    assert!(live_windows(&[no_position, no_size]).is_empty());
}

#[test]
fn falls_back_when_names_are_missing() {
    let no_bundle = RawWindow {
        bundle_id: None,
        ..raw()
    };
    assert_eq!(live_windows(&[no_bundle])[0].bundle_id, "Ghostty");

    let anonymous = RawWindow {
        app: None,
        bundle_id: None,
        title: None,
        ..raw()
    };
    let live = live_windows(&[anonymous]);
    assert_eq!(live[0].app, "pid 7");
    assert_eq!(live[0].bundle_id, "pid 7");
    assert_eq!(live[0].title, "");

    let no_app = RawWindow { app: None, ..raw() };
    let live = live_windows(&[no_app]);
    assert_eq!(live[0].app, "pid 7");
    assert_eq!(live[0].bundle_id, "com.mitchellh.ghostty");
}

#[test]
fn types_are_cloneable_and_printable() {
    let r = raw();
    assert_eq!(r.clone(), r);
    assert!(format!("{r:?}").starts_with("RawWindow"));
    let live = live_windows(&[r]);
    assert_eq!(live.clone(), live);
    assert!(format!("{:?}", live[0]).starts_with("LiveWindow"));
}
