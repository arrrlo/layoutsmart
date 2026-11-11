use super::*;

#[test]
fn max_edges() {
    let r = Rect::new(10.0, 20.0, 30.0, 40.0);
    assert_eq!((r.max_x(), r.max_y()), (40.0, 60.0));
}

#[test]
fn integral_eq_ignores_subpixel_differences() {
    let a = Rect::new(10.2, 20.4, 300.3, 400.0);
    assert!(a.integral_eq(&Rect::new(10.0, 20.0, 300.0, 400.4)));
    assert!(!a.integral_eq(&Rect::new(11.0, 20.0, 300.0, 400.0)));
    assert!(!a.integral_eq(&Rect::new(10.0, 21.0, 300.0, 400.0)));
    assert!(!a.integral_eq(&Rect::new(10.0, 20.0, 301.0, 400.0)));
    assert!(!a.integral_eq(&Rect::new(10.0, 20.0, 300.0, 401.0)));
}

#[test]
fn intersection_area_of_overlapping_rects() {
    let a = Rect::new(0.0, 0.0, 10.0, 10.0);
    assert_eq!(a.intersection_area(&Rect::new(5.0, 5.0, 10.0, 10.0)), 25.0);
    assert_eq!(a.intersection_area(&Rect::new(-5.0, 2.0, 10.0, 3.0)), 15.0);
}

#[test]
fn intersection_area_is_zero_when_disjoint_or_touching() {
    let a = Rect::new(0.0, 0.0, 10.0, 10.0);
    assert_eq!(a.intersection_area(&Rect::new(10.0, 0.0, 5.0, 5.0)), 0.0);
    assert_eq!(a.intersection_area(&Rect::new(0.0, 20.0, 5.0, 5.0)), 0.0);
    assert_eq!(a.intersection_area(&Rect::new(-20.0, 0.0, 5.0, 5.0)), 0.0);
}

#[test]
fn intersection_clips_to_overlap() {
    let a = Rect::new(0.0, 0.0, 10.0, 10.0);
    assert_eq!(
        a.intersection(&Rect::new(5.0, -5.0, 10.0, 10.0)),
        Some(Rect::new(5.0, 0.0, 5.0, 5.0))
    );
    assert_eq!(a.intersection(&Rect::new(10.0, 0.0, 5.0, 5.0)), None);
    assert_eq!(a.intersection(&Rect::new(0.0, 10.0, 5.0, 5.0)), None);
}

#[test]
fn union_covers_both() {
    let a = Rect::new(0.0, 10.0, 10.0, 10.0);
    let b = Rect::new(-5.0, 0.0, 5.0, 40.0);
    assert_eq!(a.union(&b), Rect::new(-5.0, 0.0, 15.0, 40.0));
    assert_eq!(b.union(&a), Rect::new(-5.0, 0.0, 15.0, 40.0));
    let c = Rect::new(5.0, 20.0, 10.0, 10.0);
    assert_eq!(a.union(&c), Rect::new(0.0, 10.0, 15.0, 20.0));
}

#[test]
fn union_all_of_empty_is_none() {
    assert_eq!(Rect::union_all(std::iter::empty()), None);
    let rs = [Rect::new(0.0, 0.0, 1.0, 1.0), Rect::new(2.0, 2.0, 1.0, 1.0)];
    assert_eq!(
        Rect::union_all(rs.into_iter()),
        Some(Rect::new(0.0, 0.0, 3.0, 3.0))
    );
}

#[test]
fn size_new_and_debug() {
    let s = Size::new(3.0, 4.0);
    assert_eq!((s.width, s.height), (3.0, 4.0));
    assert_eq!(format!("{s:?}"), "Size { width: 3.0, height: 4.0 }");
    assert_eq!(s.clone(), s);
    let r = Rect::new(1.0, 2.0, 3.0, 4.0);
    assert_eq!(
        format!("{r:?}"),
        "Rect { x: 1.0, y: 2.0, width: 3.0, height: 4.0 }"
    );
    assert_eq!(r.clone(), r);
}
