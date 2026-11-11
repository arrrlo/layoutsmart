use super::*;
use crate::displays::RawDisplay;
use crate::geometry::Rect;
use crate::ports::Shortcut;

fn display(name: &str, serial: u32, uuid: Option<&str>) -> RawDisplay {
    RawDisplay {
        name: Some(name.into()),
        vendor: 1,
        model: 2,
        serial,
        uuid: uuid.map(String::from),
        bounds: Rect::new(0.0, 0.0, 100.0, 100.0),
    }
}

fn screen(uuid: &str, current: u64, spaces: &[(u64, bool)]) -> RawScreen {
    RawScreen {
        uuid: uuid.into(),
        current,
        spaces: spaces
            .iter()
            .map(|(id, fullscreen)| RawSpace {
                id: *id,
                fullscreen: *fullscreen,
            })
            .collect(),
    }
}

/// Main: desktop 1, [desktop 2], a full-screen app, desktop 3.  Side: [full-screen app], desktop 1.
fn two_screens() -> DesktopMap {
    DesktopMap::new(
        &[
            display("Main", 10, Some("U-MAIN")),
            display("Side", 20, Some("U-SIDE")),
        ],
        &[
            screen(
                "U-MAIN",
                12,
                &[(11, false), (12, false), (13, true), (14, false)],
            ),
            screen("U-SIDE", 21, &[(21, true), (22, false)]),
        ],
    )
}

fn place(display: &str, name: &str, kind: Kind, row: usize, showing: bool) -> Place {
    Place {
        display: display.into(),
        display_name: name.into(),
        kind,
        row,
        showing,
        everywhere: false,
    }
}

#[test]
fn ordinary_desktops_are_numbered_per_screen_and_full_screen_ones_are_not() {
    let map = two_screens();
    assert_eq!(
        map.place(&[11]),
        Some(place("1-2-10", "Main", Kind::Desktop(1), 0, false))
    );
    assert_eq!(
        map.place(&[12]),
        Some(place("1-2-10", "Main", Kind::Desktop(2), 1, true))
    );
    assert_eq!(
        map.place(&[13]),
        Some(place("1-2-10", "Main", Kind::FullScreen, 2, false))
    );
    assert_eq!(
        map.place(&[14]),
        Some(place("1-2-10", "Main", Kind::Desktop(3), 3, false))
    );
    assert_eq!(
        map.place(&[21]),
        Some(place("1-2-20", "Side", Kind::FullScreen, 0, true))
    );
    assert_eq!(
        map.place(&[22]),
        Some(place("1-2-20", "Side", Kind::Desktop(1), 1, false))
    );
}

#[test]
fn a_window_with_no_known_desktop_has_no_place() {
    let map = two_screens();
    assert_eq!(map.place(&[]), None);
    assert_eq!(map.place(&[99]), None);
    assert!(!map.place(&[99, 14]).unwrap().everywhere);
}

#[test]
fn rows_list_each_screens_desktops_in_order() {
    let map = two_screens();
    assert_eq!(
        map.rows(),
        [
            ScreenRow {
                display: "1-2-10".into(),
                name: "Main".into(),
                kinds: vec![
                    Kind::Desktop(1),
                    Kind::Desktop(2),
                    Kind::FullScreen,
                    Kind::Desktop(3)
                ],
                showing: Some(1),
            },
            ScreenRow {
                display: "1-2-20".into(),
                name: "Side".into(),
                kinds: vec![Kind::FullScreen, Kind::Desktop(1)],
                showing: Some(0),
            },
        ]
    );
    assert_eq!(map.row("1-2-20").map(|row| row.name.as_str()), Some("Side"));
    assert_eq!(map.row("nope"), None);
}

#[test]
fn known_only_when_macos_reported_desktops() {
    assert!(two_screens().is_known());
    assert!(!DesktopMap::new(&[display("Main", 10, Some("U-MAIN"))], &[]).is_known());
    assert!(!DesktopMap::default().is_known());
    assert_eq!(DesktopMap::default().place(&[1]), None);
    assert!(DesktopMap::default().rows().is_empty());
}

#[test]
fn a_screen_that_matches_no_display_is_named_by_its_identifier() {
    let map = DesktopMap::new(
        &[
            display("Main", 10, None),
            display("Other", 20, Some("U-OTHER")),
        ],
        &[screen("Main", 5, &[(5, false)])],
    );
    assert_eq!(
        map.place(&[5]),
        Some(place("Main", "Main", Kind::Desktop(1), 0, true))
    );
    assert_eq!(map.rows()[0].display, "Main");
}

#[test]
fn a_display_without_a_name_is_called_by_its_position() {
    let mut nameless = display("ignored", 20, Some("U-SIDE"));
    nameless.name = None;
    let map = DesktopMap::new(
        &[display("Main", 10, Some("U-MAIN")), nameless],
        &[screen("U-SIDE", 1, &[(1, false)])],
    );
    assert_eq!(
        map.place(&[1]),
        Some(place("1-2-20", "Display 2", Kind::Desktop(1), 0, true))
    );
}

#[test]
fn nothing_is_marked_showing_when_the_current_desktop_is_not_in_the_row() {
    let map = DesktopMap::new(
        &[display("Main", 10, Some("U-MAIN"))],
        &[screen("U-MAIN", 77, &[(1, false), (2, false)])],
    );
    assert_eq!(map.rows()[0].showing, None);
    assert!(!map.place(&[1]).unwrap().showing);
    assert!(!map.place(&[2]).unwrap().showing);
}

#[test]
fn labels() {
    assert_eq!(Kind::Desktop(3).label(), "desktop 3");
    assert_eq!(Kind::FullScreen.label(), "full screen");
    assert!(Kind::FullScreen.is_fullscreen());
    assert!(!Kind::Desktop(1).is_fullscreen());
    assert_eq!(
        place("id", "Main", Kind::Desktop(2), 1, true).label(),
        "Main · desktop 2"
    );
    assert_eq!(
        place("id", "Side", Kind::FullScreen, 0, false).label(),
        "Side · full screen (hidden)"
    );
}

#[test]
fn types_are_cloneable_and_printable() {
    let map = two_screens();
    assert_eq!(map.clone(), map);
    assert!(format!("{map:?}").starts_with("DesktopMap"));
    let raw = screen("U", 1, &[(1, false)]);
    assert_eq!(raw.clone(), raw);
    assert!(format!("{raw:?}").starts_with("RawScreen"));
    assert!(format!("{:?}", raw.spaces[0]).starts_with("RawSpace"));
    let row = map.rows()[0].clone();
    assert_eq!(row, map.rows()[0]);
    assert!(format!("{row:?}").starts_with("ScreenRow"));
    let p = map.place(&[11]).unwrap();
    assert_eq!(p.clone(), p);
    assert!(format!("{p:?}").starts_with("Place"));
    assert_eq!(format!("{:?}", Kind::Desktop(1)), "Desktop(1)");
}

#[test]
fn a_window_on_several_desktops_is_everywhere_and_counts_as_being_in_view() {
    let map = two_screens();
    // One of its desktops is in view: that is where it is.
    assert_eq!(
        map.place(&[11, 12, 14]),
        Some(Place {
            everywhere: true,
            ..place("1-2-10", "Main", Kind::Desktop(2), 1, true)
        })
    );
    // None is in view: the first known one.
    assert_eq!(
        map.place(&[99, 14, 11]),
        Some(Place {
            everywhere: true,
            ..place("1-2-10", "Main", Kind::Desktop(3), 3, false)
        })
    );
    assert_eq!(map.place(&[11, 12]).unwrap().label(), "Main · all desktops");
}

#[test]
fn a_place_holds_its_own_desktop_number_only_unless_it_is_everywhere() {
    let two = place("d", "D", Kind::Desktop(2), 1, true);
    assert!(two.holds(2));
    assert!(!two.holds(3));
    assert!(!place("d", "D", Kind::FullScreen, 1, true).holds(1));
    let everywhere = Place {
        everywhere: true,
        ..two
    };
    assert!(everywhere.holds(3));
}

#[test]
fn a_row_counts_and_finds_its_ordinary_desktops() {
    let map = two_screens();
    let main = map.row("1-2-10").unwrap();
    assert_eq!(main.desktops(), 3);
    assert_eq!(main.row_of(1), Some(0));
    assert_eq!(main.row_of(3), Some(3));
    assert_eq!(main.row_of(4), None);
    let side = map.row("1-2-20").unwrap();
    assert_eq!(side.desktops(), 1);
    assert_eq!(side.row_of(1), Some(1));
}

fn shortcut(enabled: Option<bool>, key: Option<i64>, modifiers: Option<i64>) -> Shortcut {
    Shortcut {
        enabled,
        key,
        modifiers,
    }
}

#[test]
fn desktops_can_be_stepped_through_only_with_the_standard_shortcuts() {
    let untouched = shortcut(None, None, None);
    assert!(steps_with_control_arrows(&[untouched, untouched]));
    // As macOS writes them once the settings pane has been opened.
    let left = shortcut(Some(true), Some(123), Some(8_650_752));
    let right = shortcut(Some(true), Some(124), Some(8_650_752));
    assert!(steps_with_control_arrows(&[left, right]));
    // Older systems store Control alone.
    let plain = shortcut(Some(true), Some(123), Some(262_144));
    assert!(steps_with_control_arrows(&[plain, right]));

    let off = shortcut(Some(false), Some(123), Some(8_650_752));
    assert!(!steps_with_control_arrows(&[off, right]));
    assert!(!steps_with_control_arrows(&[
        left,
        shortcut(Some(false), None, None)
    ]));
    // Another key.
    assert!(!steps_with_control_arrows(&[right, right]));
    assert!(!steps_with_control_arrows(&[left, left]));
    // Other modifier keys: Control with Shift, with Option, with Command; Command alone;
    // none at all.
    for flags in [393_216, 786_432, 1_310_720, 1_048_576, 0] {
        let changed = shortcut(Some(true), Some(124), Some(flags));
        assert!(!steps_with_control_arrows(&[left, changed]), "{flags}");
    }
}
