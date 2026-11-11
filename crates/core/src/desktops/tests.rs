use super::*;
use crate::displays::RawDisplay;
use crate::geometry::Rect;

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
    // Several ids: the first one that is known decides.
    assert_eq!(map.place(&[99, 14, 11]).unwrap().kind, Kind::Desktop(3));
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
