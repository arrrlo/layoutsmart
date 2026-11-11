use super::*;
use crate::testing::FakeRunner;
use std::fs;

const CLEAN: &str = r#"
use objc2::define_class;

fn ordinary(x: Option<u8>) -> u8 {
    if let Some(x) = x { x } else { 0 }
}

define_class!(
    #[unsafe(super(NSObject))]
    pub struct Controller;

    impl Controller {
        #[unsafe(method(menuItemClicked:))]
        fn menu_item_clicked(&self, sender: &NSMenuItem) {
            status::clicked(app(), sender.tag())
        }

        #[unsafe(method(isFlipped))]
        fn is_flipped(&self) -> bool {
            true
        }
    }

    unsafe impl NSObjectProtocol for Controller {}

    unsafe impl NSMenuDelegate for Controller {
        #[unsafe(method(menuNeedsUpdate:))]
        fn menu_needs_update(&self, menu: &NSMenu) {
            status::rebuild(app(), &menu, |item| item.tag())
        }
    }
);
"#;

fn class_with_body(body: &str) -> String {
    format!(
        "define_class!(\n    struct View;\n    impl View {{\n        fn method(&self) {{\n            {body}\n        }}\n    }}\n);\n"
    )
}

fn words(source: &str) -> Vec<String> {
    decisions_in_class_macros(source)
        .into_iter()
        .map(|finding| finding.what)
        .collect()
}

#[test]
fn single_call_methods_are_fine() {
    assert_eq!(decisions_in_class_macros(CLEAN), []);
}

#[test]
fn every_kind_of_decision_is_reported() {
    for (body, expected) in [
        ("if ready() { go() }", "if"),
        ("match x { _ => go() }", "match"),
        ("while busy() { wait() }", "while"),
        ("for item in items() { use_it(item) }", "for"),
        ("loop { spin() }", "loop"),
        ("let value = fallible()?; use_it(value)", "?"),
        ("a() && b()", "&&"),
        ("a() || b()", "||"),
    ] {
        assert_eq!(words(&class_with_body(body)), [expected], "{body}");
    }
}

#[test]
fn reports_the_line() {
    let findings = decisions_in_class_macros(&class_with_body("if a() { b() }"));
    assert_eq!(
        findings,
        [Finding {
            line: 5,
            what: "if".into()
        }]
    );
}

#[test]
fn decisions_inside_parentheses_and_nested_blocks_are_found() {
    assert_eq!(
        words(&class_with_body("call((if a() { 1 } else { 2 }))")),
        ["if"]
    );
    assert_eq!(words(&class_with_body("{ { while a() {} } }")), ["while"]);
    assert_eq!(words(&class_with_body("items.map(|x| x? + 1)")), ["?"]);
}

#[test]
fn several_decisions_are_all_reported() {
    assert_eq!(
        words(&class_with_body("if a() && b() { c()? }")),
        ["if", "&&", "?"]
    );
}

#[test]
fn single_ampersands_and_closures_are_not_decisions() {
    assert_eq!(
        words(&class_with_body(
            "call(&self.x, |item| item.tag(), a & b, c | d)"
        )),
        [] as [&str; 0]
    );
}

#[test]
fn a_reference_to_a_closure_is_not_a_decision() {
    // `&|` is two different symbols written together, not `&&` or `||`.
    assert_eq!(
        words(&class_with_body("call(&|item| item.tag())")),
        [] as [&str; 0]
    );
    assert_eq!(words(&class_with_body("call(*&value)")), [] as [&str; 0]);
}

#[test]
fn signatures_and_attributes_are_not_method_bodies() {
    let source = "define_class!(\n    struct View;\n    impl View {\n        \
                  #[cfg(any(a, b))]\n        \
                  fn method<T: ?Sized>(&self, names: &&T) { call(names) }\n    }\n);";
    assert_eq!(decisions_in_class_macros(source), []);
}

#[test]
fn impl_headers_and_code_outside_the_macro_are_ignored() {
    let source = "fn outside() { if a() && b() { loop {} } }\n\
                  define_class!(struct A; unsafe impl P for A {} unsafe impl Q for A {});";
    assert_eq!(decisions_in_class_macros(source), []);
}

#[test]
fn macros_inside_modules_and_functions_are_found() {
    let source = format!(
        "mod inner {{ fn f() {{ {} }} }}",
        class_with_body("loop {}")
    );
    assert_eq!(words(&source), ["loop"]);
}

#[test]
fn other_macros_are_not_checked() {
    let source = "vec!(impl X { fn f() { if a() {} } }); println!(\"{}\", if a { 1 } else { 2 });";
    assert_eq!(decisions_in_class_macros(source), []);
}

#[test]
fn source_that_cannot_be_read_is_a_finding() {
    let findings = decisions_in_class_macros("define_class!( fn broken( { ");
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].what, "unreadable source");
}

fn tree() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir_all(dir.path().join("app/ui")).unwrap();
    fs::create_dir_all(dir.path().join("macos")).unwrap();
    fs::write(dir.path().join("app/ui/controller.rs"), CLEAN).unwrap();
    fs::write(dir.path().join("app/lib.rs"), "pub mod ui;").unwrap();
    fs::write(dir.path().join("macos/ax.rs"), "fn f() { if a() {} }").unwrap();
    fs::write(dir.path().join("app/notes.txt"), class_with_body("loop {}")).unwrap();
    dir
}

#[test]
fn a_clean_tree_passes() {
    let dir = tree();
    let runner = FakeRunner::default();
    assert!(check(
        &runner,
        &[dir.path().join("app"), dir.path().join("macos")]
    ));
    assert_eq!(
        runner.said(),
        "▶ lint-shell: methods inside define_class! must be single calls"
    );
}

#[test]
fn a_decision_in_any_checked_file_fails_and_is_listed() {
    let dir = tree();
    let bad = dir.path().join("app/ui/view.rs");
    fs::write(&bad, class_with_body("if a() { b()? }")).unwrap();
    let runner = FakeRunner::default();
    assert!(!check(
        &runner,
        &[dir.path().join("macos"), dir.path().join("app")]
    ));
    let said = runner.said();
    assert!(
        said.contains(&format!("✗ {}:5: `if` inside define_class!", bad.display())),
        "{said}"
    );
    assert!(
        said.contains(&format!("✗ {}:5: `?` inside define_class!", bad.display())),
        "{said}"
    );
    assert_eq!(said.lines().count(), 3);
}

#[test]
fn a_missing_folder_is_not_an_error() {
    let dir = tempfile::tempdir().unwrap();
    assert!(check(&FakeRunner::default(), &[dir.path().join("nope")]));
}

#[test]
fn finding_is_cloneable_and_printable() {
    let finding = Finding {
        line: 1,
        what: "if".into(),
    };
    assert_eq!(finding.clone(), finding);
    assert!(format!("{finding:?}").starts_with("Finding"));
}
