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
        }],
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
