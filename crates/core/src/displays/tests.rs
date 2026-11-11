use super::*;
use crate::geometry::Rect;

fn raw(name: Option<&str>, vendor: u32, model: u32, serial: u32) -> RawDisplay {
    RawDisplay {
        name: name.map(String::from),
        vendor,
        model,
        serial,
        uuid: None,
        bounds: Rect::new(-5.0, 7.0, 10.0, 20.0),
    }
}

fn info(id: &str) -> DisplayInfo {
    DisplayInfo {
        name: "x".into(),
        id: id.into(),
        x: 0.0,
        y: 0.0,
        width: 1.0,
        height: 1.0,
    }
}

#[test]
fn identity_format() {
    assert_eq!(
        display_identity(4268, 53360, 123456789),
        "4268-53360-123456789"
    );
}

#[test]
fn converts_raw_displays_with_fallback_names() {
    let infos = display_infos(&[raw(Some("DELL"), 1, 2, 3), raw(None, 4, 5, 6)]);
    assert_eq!(infos.len(), 2);
    assert_eq!(infos[0].name, "DELL");
    assert_eq!(infos[0].id, "1-2-3");
    assert_eq!(infos[1].name, "Display 2");
    assert_eq!(infos[1].id, "4-5-6");
    assert_eq!(infos[1].frame(), Rect::new(-5.0, 7.0, 10.0, 20.0));
}

#[test]
fn same_set_ignores_order() {
    let a = [info("1-1-1"), info("2-2-2"), info("3-3-3")];
    let b = [info("3-3-3"), info("1-1-1"), info("2-2-2")];
    assert!(same_display_set(&a, &b));
    assert!(same_display_set(&[], &[]));
}

#[test]
fn different_sets_differ() {
    let a = [info("1-1-1"), info("2-2-2")];
    assert!(!same_display_set(&a, &[info("1-1-1"), info("9-9-9")]));
    assert!(!same_display_set(&a, &a[..1]));
    assert!(!same_display_set(&a[..1], &a));
    assert!(!same_display_set(&a, &[]));
}

#[test]
fn identical_monitors_are_counted() {
    let two = [info("1-1-0"), info("1-1-0")];
    assert!(same_display_set(&two, &two));
    assert!(!same_display_set(&two, &two[..1]));
    assert!(!same_display_set(&two, &[info("1-1-0"), info("2-2-0")]));
}

#[test]
fn raw_display_is_cloneable_and_printable() {
    let r = raw(None, 1, 2, 3);
    assert_eq!(r.clone(), r);
    assert!(format!("{r:?}").starts_with("RawDisplay"));
}
