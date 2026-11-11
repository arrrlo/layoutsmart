use super::*;
use crate::geometry::Rect;

/// Exactly what the Swift tool wrote (sorted keys, pretty-printed).
const SWIFT_JSON: &str = r#"{
  "displays" : [
{ "height" : 1440, "id" : "4268-53360-123456789", "name" : "Studio Display", "width" : 2560, "x" : 0, "y" : 0 }
  ],
  "profile" : "home",
  "savedAt" : "2026-11-11T09:42:04Z",
  "windows" : [
{ "app" : "Ghostty", "bundleId" : "com.mitchellh.ghostty", "height" : 800, "title" : "zsh", "width" : 1200, "x" : 10.5, "y" : 40 }
  ]
}"#;

fn sample() -> Layout {
    Layout {
        profile: "home".into(),
        saved_at: "2026-11-11T09:42:04Z".into(),
        displays: vec![DisplayInfo {
            name: "Studio Display".into(),
            id: "4268-53360-123456789".into(),
            x: 0.0,
            y: 0.0,
            width: 2560.0,
            height: 1440.0,
        }],
        windows: vec![WindowInfo {
            app: "Ghostty".into(),
            bundle_id: "com.mitchellh.ghostty".into(),
            title: "zsh".into(),
            x: 10.5,
            y: 40.0,
            width: 1200.0,
            height: 800.0,
            display: None,
            desktop: None,
            fullscreen: false,
        }],
        desktops: Vec::new(),
    }
}

#[test]
fn reads_swift_format() {
    assert_eq!(Layout::from_json(SWIFT_JSON).unwrap(), sample());
}

#[test]
fn json_round_trip_uses_camel_case_keys() {
    let json = sample().to_json();
    assert!(json.contains("\"savedAt\""));
    assert!(json.contains("\"bundleId\""));
    assert_eq!(Layout::from_json(&json).unwrap(), sample());
}

#[test]
fn rejects_garbage() {
    assert!(Layout::from_json("{").is_err());
}

#[test]
fn frames() {
    let l = sample();
    assert_eq!(l.displays[0].frame(), Rect::new(0.0, 0.0, 2560.0, 1440.0));
    assert_eq!(l.windows[0].frame(), Rect::new(10.5, 40.0, 1200.0, 800.0));
}

#[test]
fn window_from_frame() {
    let w = WindowInfo::new("A", "a.b", "t", Rect::new(1.0, 2.0, 3.0, 4.0));
    assert_eq!(
        (w.app.as_str(), w.bundle_id.as_str(), w.title.as_str()),
        ("A", "a.b", "t")
    );
    assert_eq!(w.frame(), Rect::new(1.0, 2.0, 3.0, 4.0));
}

#[test]
fn debug_and_clone() {
    let l = sample();
    assert_eq!(l.clone(), l);
    assert!(format!("{l:?}").starts_with("Layout { profile: \"home\""));
}

#[test]
fn a_plain_window_is_written_exactly_as_before() {
    // Profiles written by 0.2 for ordinary setups stay readable by 0.1.
    let json = sample().to_json();
    for new_key in ["display\"", "desktop", "fullscreen", "desktops", "showing"] {
        assert!(!json.contains(new_key), "{new_key} in {json}");
    }
}

#[test]
fn a_window_remembers_its_screen_desktop_and_full_screen_state() {
    let frame = Rect::new(1.0, 2.0, 3.0, 4.0);
    let placed = WindowInfo::new("A", "a.b", "t", frame).on_desktop("1-2-3", 2);
    assert_eq!(
        (placed.display.as_deref(), placed.desktop, placed.fullscreen),
        (Some("1-2-3"), Some(2), false)
    );
    let loose = WindowInfo::new("A", "a.b", "t", frame).on_display("1-2-3");
    assert_eq!(
        (loose.display.as_deref(), loose.desktop, loose.fullscreen),
        (Some("1-2-3"), None, false)
    );
    let full = WindowInfo::new("A", "a.b", "t", frame).in_full_screen("1-2-3");
    assert_eq!(
        (full.display.as_deref(), full.desktop, full.fullscreen),
        (Some("1-2-3"), None, true)
    );

    let mut layout = sample();
    layout.windows = vec![placed, full];
    let json = layout.to_json();
    assert!(json.contains("\"display\": \"1-2-3\""), "{json}");
    assert!(json.contains("\"desktop\": 2"), "{json}");
    assert!(json.contains("\"fullscreen\": true"), "{json}");
    assert_eq!(Layout::from_json(&json).unwrap(), layout);
}

#[test]
fn a_profile_remembers_each_screens_desktops_and_what_was_in_view() {
    let mut layout = sample();
    layout.desktops = vec![
        SavedScreen {
            display: "1-2-3".into(),
            desktops: 4,
            showing: Showing::Desktop(2),
        },
        SavedScreen {
            display: "4-5-6".into(),
            desktops: 1,
            showing: Showing::FullScreen {
                bundle_id: "com.mitchellh.ghostty".into(),
                title: "zsh".into(),
            },
        },
        SavedScreen {
            display: "7-8-9".into(),
            desktops: 1,
            showing: Showing::Unknown,
        },
    ];
    let json = layout.to_json();
    assert!(json.contains("\"desktop\": 2"), "{json}");
    assert!(json.contains("\"fullScreen\""), "{json}");
    assert!(
        json.contains("\"bundleId\": \"com.mitchellh.ghostty\""),
        "{json}"
    );
    assert!(json.contains("\"unknown\""), "{json}");
    assert_eq!(Layout::from_json(&json).unwrap(), layout);
    assert_eq!(layout.screen("4-5-6"), Some(&layout.desktops[1]));
    assert_eq!(layout.screen("nope"), None);
    assert!(layout.knows_desktops());
    assert!(!sample().knows_desktops());
}

#[test]
fn new_types_are_cloneable_and_printable() {
    let screen = SavedScreen {
        display: "d".into(),
        desktops: 1,
        showing: Showing::Desktop(1),
    };
    assert_eq!(screen.clone(), screen);
    assert!(format!("{screen:?}").starts_with("SavedScreen"));
    assert_eq!(format!("{:?}", Showing::Unknown), "Unknown");
}
