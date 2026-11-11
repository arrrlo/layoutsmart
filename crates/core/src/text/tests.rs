use super::*;

#[test]
fn plural_adds_an_s_except_for_one() {
    assert_eq!(plural(0, "window"), "0 windows");
    assert_eq!(plural(1, "window"), "1 window");
    assert_eq!(plural(2, "display"), "2 displays");
    assert_eq!(plural(12, "window"), "12 windows");
}

#[test]
fn quoted_uses_curly_quotes() {
    assert_eq!(quoted("home"), "“home”");
}
