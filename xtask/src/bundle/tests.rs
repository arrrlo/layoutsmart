use super::*;
use crate::testing::FakeRunner;

const GOOD_LIBS: &str = "\
dist/Layoutsmart.app/Contents/MacOS/Layoutsmart (architecture x86_64):
\t/System/Library/Frameworks/AppKit.framework/Versions/C/AppKit (compatibility version 45.0.0, current version 2575.20.19)
\t/usr/lib/libobjc.A.dylib (compatibility version 1.0.0, current version 228.0.0)
\t/usr/lib/libSystem.B.dylib (compatibility version 1.0.0, current version 1351.0.0)
dist/Layoutsmart.app/Contents/MacOS/Layoutsmart (architecture arm64):
\t/System/Library/Frameworks/AppKit.framework/Versions/C/AppKit (compatibility version 45.0.0, current version 2575.20.19)
\t/usr/lib/libSystem.B.dylib (compatibility version 1.0.0, current version 1351.0.0)
";

fn good_machine() -> FakeRunner {
    FakeRunner {
        outputs: vec![("lipo -archs", "x86_64 arm64\n"), ("otool -L", GOOD_LIBS)],
        ..Default::default()
    }
}

fn ad_hoc() -> Signing {
    Signing::from_env(|_| None)
}

#[test]
fn signing_is_ad_hoc_unless_an_identity_is_given() {
    assert_eq!(
        ad_hoc(),
        Signing {
            identity: "-".into(),
            keychain: None
        }
    );
    let blank = Signing::from_env(|key| (key == "LAYOUTSMART_SIGN_IDENTITY").then(String::new));
    assert_eq!(blank.identity, "-");
}

#[test]
fn signing_reads_identity_and_keychain_from_the_environment() {
    let signing = Signing::from_env(|key| match key {
        "LAYOUTSMART_SIGN_IDENTITY" => Some("Layoutsmart Self-Signed".into()),
        "LAYOUTSMART_SIGN_KEYCHAIN" => Some("/tmp/signing.keychain-db".into()),
        _ => None,
    });
    assert_eq!(
        signing,
        Signing {
            identity: "Layoutsmart Self-Signed".into(),
            keychain: Some("/tmp/signing.keychain-db".into()),
        }
    );
    let no_keychain = Signing::from_env(|key| match key {
        "LAYOUTSMART_SIGN_IDENTITY" => Some("Me".into()),
        "LAYOUTSMART_SIGN_KEYCHAIN" => Some(String::new()),
        _ => None,
    });
    assert_eq!(no_keychain.keychain, None);
}

#[test]
fn info_plist_is_filled_in() {
    let plist = info_plist("1.2.3");
    assert_eq!(plist.matches("<string>1.2.3</string>").count(), 2);
    assert!(plist.contains("<string>io.github.arrrlo.layoutsmart</string>"));
    assert!(plist.contains("<key>LSMinimumSystemVersion</key>\n    <string>13.0</string>"));
    assert!(plist.contains("<key>LSUIElement</key>\n    <true/>"));
    assert!(plist.contains("<key>CFBundleExecutable</key>\n    <string>Layoutsmart</string>"));
    assert!(!plist.contains('@'));
}

#[test]
fn render_replaces_every_placeholder() {
    assert_eq!(
        render("@A@-@B@-@A@", &[("@A@", "1"), ("@B@", "2")]),
        "1-2-1"
    );
    assert_eq!(render("plain", &[("@A@", "1")]), "plain");
}

#[test]
fn universal_means_both_architectures() {
    assert!(is_universal("x86_64 arm64\n"));
    assert!(is_universal("arm64 x86_64"));
    assert!(!is_universal("arm64\n"));
    assert!(!is_universal("x86_64\n"));
    assert!(!is_universal("arm64e x86_64h\n"));
    assert!(!is_universal(""));
}

#[test]
fn only_libraries_that_ship_with_macos_are_allowed() {
    assert_eq!(foreign_libraries(GOOD_LIBS), [] as [&str; 0]);
    let bad = format!(
        "{GOOD_LIBS}\t/opt/homebrew/lib/libfoo.1.dylib (compatibility version 1.0.0)\n\
         \t@rpath/libbar.dylib (compatibility version 1.0.0)\n\
         \t/usr/local/lib/libbaz.dylib (compatibility version 1.0.0)\n"
    );
    assert_eq!(
        foreign_libraries(&bad),
        [
            "/opt/homebrew/lib/libfoo.1.dylib",
            "@rpath/libbar.dylib",
            "/usr/local/lib/libbaz.dylib"
        ]
    );
    assert_eq!(foreign_libraries(""), [] as [&str; 0]);
}

#[test]
fn bundle_builds_assembles_signs_and_verifies() {
    let runner = good_machine();
    assert!(bundle(&runner, &ad_hoc()));
    let app = "dist/Layoutsmart.app";
    let arm = "target/aarch64-apple-darwin/release";
    let intel = "target/x86_64-apple-darwin/release";
    let log = runner.log();
    let expected = [
        "MACOSX_DEPLOYMENT_TARGET=13.0 cargo build --release -p layoutsmart --target aarch64-apple-darwin".to_string(),
        "MACOSX_DEPLOYMENT_TARGET=13.0 cargo build --release -p layoutsmart --target x86_64-apple-darwin".to_string(),
        format!("rm -rf {app}"),
        format!("mkdir -p {app}/Contents/MacOS {app}/Contents/Resources"),
        format!("lipo -create -output {app}/Contents/MacOS/Layoutsmart {arm}/layoutsmart-app {intel}/layoutsmart-app"),
        format!("lipo -create -output {app}/Contents/MacOS/layoutsmart-cli {arm}/layoutsmart-cli {intel}/layoutsmart-cli"),
        format!("cp packaging/AppIcon.icns {app}/Contents/Resources/AppIcon.icns"),
        format!("write {app}/Contents/Info.plist\n{}", info_plist(VERSION)),
        format!("codesign --force --sign - --identifier io.github.arrrlo.layoutsmart.cli {app}/Contents/MacOS/layoutsmart-cli"),
        format!("codesign --force --sign - {app}"),
        format!("lipo -archs {app}/Contents/MacOS/Layoutsmart"),
        format!("otool -L {app}/Contents/MacOS/Layoutsmart"),
        format!("lipo -archs {app}/Contents/MacOS/layoutsmart-cli"),
        format!("otool -L {app}/Contents/MacOS/layoutsmart-cli"),
        format!("codesign --verify --deep --strict {app}"),
    ];
    assert_eq!(log.len(), expected.len(), "{log:#?}");
    for (got, want) in log.iter().zip(&expected) {
        assert_eq!(got, want);
    }
}

#[test]
fn a_named_identity_and_keychain_are_passed_to_codesign() {
    let runner = good_machine();
    let signing = Signing {
        identity: "Layoutsmart Self-Signed".into(),
        keychain: Some("/k/signing.keychain-db".into()),
    };
    assert!(bundle(&runner, &signing));
    let signs: Vec<String> = runner
        .log()
        .into_iter()
        .filter(|c| c.starts_with("codesign --force"))
        .collect();
    assert_eq!(signs.len(), 2);
    for sign in signs {
        // The identity contains a space, so it must stay one argument.
        assert!(
            sign.contains("--sign Layoutsmart Self-Signed --keychain /k/signing.keychain-db"),
            "{sign}"
        );
    }
}

#[test]
fn bundle_stops_when_a_step_fails() {
    for (failing, steps_run) in [
        ("--target x86_64-apple-darwin", 2),
        ("lipo -create", 5),
        ("write dist/Layoutsmart.app/Contents/Info.plist", 8),
        ("codesign --force", 9),
        ("codesign --verify", 15),
    ] {
        let runner = FakeRunner {
            fail_on: Some(failing),
            ..good_machine()
        };
        assert!(!bundle(&runner, &ad_hoc()), "{failing}");
        assert_eq!(runner.log().len(), steps_run, "{failing}");
    }
}

#[test]
fn bundle_refuses_a_binary_that_is_not_universal() {
    let runner = FakeRunner {
        outputs: vec![("lipo -archs", "arm64\n"), ("otool -L", GOOD_LIBS)],
        ..Default::default()
    };
    assert!(!bundle(&runner, &ad_hoc()));
    assert!(runner.said().contains(
        "✗ dist/Layoutsmart.app/Contents/MacOS/Layoutsmart is not a universal binary (found: arm64)"
    ));
    assert!(
        !runner
            .log()
            .iter()
            .any(|c| c.starts_with("codesign --verify"))
    );
}

#[test]
fn bundle_refuses_libraries_that_do_not_ship_with_macos() {
    let runner = FakeRunner {
        outputs: vec![
            ("lipo -archs", "x86_64 arm64\n"),
            (
                "otool -L",
                "bin:\n\t/opt/homebrew/lib/libfoo.dylib (x)\n\t/usr/lib/libSystem.B.dylib (y)\n",
            ),
        ],
        ..Default::default()
    };
    assert!(!bundle(&runner, &ad_hoc()));
    assert!(runner.said().contains(
        "✗ dist/Layoutsmart.app/Contents/MacOS/Layoutsmart needs libraries that are not part of macOS: /opt/homebrew/lib/libfoo.dylib"
    ));
}

#[test]
fn bundle_fails_if_a_binary_cannot_be_inspected() {
    for tool in ["lipo -archs", "otool -L"] {
        let runner = FakeRunner {
            fail_on: Some(tool),
            ..good_machine()
        };
        assert!(!bundle(&runner, &ad_hoc()), "{tool}");
        assert!(
            runner
                .said()
                .contains("✗ dist/Layoutsmart.app/Contents/MacOS/Layoutsmart"),
            "{tool}"
        );
    }
}

#[test]
fn signing_is_cloneable_and_printable() {
    let signing = ad_hoc();
    assert_eq!(signing.clone(), signing);
    assert!(format!("{signing:?}").starts_with("Signing"));
}
