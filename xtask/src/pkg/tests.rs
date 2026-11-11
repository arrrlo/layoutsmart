use super::*;
use crate::bundle::{Signing, VERSION};
use crate::testing::FakeRunner;

fn good_machine() -> FakeRunner {
    FakeRunner {
        outputs: vec![
            ("lipo -archs", "x86_64 arm64\n"),
            ("otool -L", "bin:\n\t/usr/lib/libSystem.B.dylib (x)\n"),
        ],
        ..Default::default()
    }
}

fn ad_hoc() -> Signing {
    Signing::from_env(|_| None)
}

#[test]
fn installer_file_name_carries_the_version() {
    assert_eq!(installer_path("1.2.3"), "dist/Layoutsmart-1.2.3.pkg");
}

#[test]
fn distribution_is_filled_in() {
    let xml = distribution("1.2.3");
    assert!(xml.contains("<os-version min=\"13.0\"/>"));
    assert!(xml.contains("hostArchitectures=\"arm64,x86_64\""));
    assert!(xml.contains(
        "<pkg-ref id=\"io.github.arrrlo.layoutsmart\" version=\"1.2.3\" onConclusion=\"none\">component.pkg</pkg-ref>"
    ));
    assert!(xml.contains("<conclusion file=\"conclusion.html\""));
    assert!(!xml.contains('@'));
}

#[test]
fn pkg_bundles_then_builds_the_installer() {
    let runner = good_machine();
    assert!(pkg(&runner, &ad_hoc()));
    let log = runner.log();
    let after_bundle: Vec<&String> = log.iter().skip(15).collect();
    let expected = [
        "rm -rf dist/pkgroot".to_string(),
        "mkdir -p dist/pkgroot".to_string(),
        "ditto dist/Layoutsmart.app dist/pkgroot/Layoutsmart.app".to_string(),
        format!(
            "pkgbuild --root dist/pkgroot --component-plist packaging/component.plist \
             --identifier io.github.arrrlo.layoutsmart --version {VERSION} \
             --install-location /Applications --scripts packaging/scripts dist/component.pkg"
        ),
        format!("write dist/distribution.xml\n{}", distribution(VERSION)),
        format!(
            "productbuild --distribution dist/distribution.xml --package-path dist \
             --resources packaging/resources dist/Layoutsmart-{VERSION}.pkg"
        ),
        // Unpack the finished installer and check the app inside is intact.
        "rm -rf dist/pkgcheck".to_string(),
        format!("pkgutil --expand-full dist/Layoutsmart-{VERSION}.pkg dist/pkgcheck"),
        "codesign --verify --deep --strict dist/pkgcheck/component.pkg/Payload/Layoutsmart.app"
            .to_string(),
        "rm -rf dist/pkgroot dist/component.pkg dist/distribution.xml dist/pkgcheck".to_string(),
    ];
    assert_eq!(after_bundle.len(), expected.len(), "{after_bundle:#?}");
    for (got, want) in after_bundle.iter().zip(&expected) {
        assert_eq!(*got, want);
    }
    assert!(
        runner
            .said()
            .ends_with(&format!("✓ dist/Layoutsmart-{VERSION}.pkg"))
    );
}

#[test]
fn pkg_does_nothing_more_when_the_bundle_fails() {
    let runner = FakeRunner {
        fail_on: Some("codesign --verify"),
        ..good_machine()
    };
    assert!(!pkg(&runner, &ad_hoc()));
    assert_eq!(runner.log().len(), 15);
}

#[test]
fn pkg_stops_when_a_step_fails() {
    for (failing, steps_after_bundle) in [
        ("pkgbuild", 4),
        ("write dist/distribution.xml", 5),
        ("productbuild", 6),
        ("pkgutil --expand-full", 8),
        ("Payload/Layoutsmart.app", 9),
    ] {
        let runner = FakeRunner {
            fail_on: Some(failing),
            ..good_machine()
        };
        assert!(!pkg(&runner, &ad_hoc()), "{failing}");
        assert_eq!(runner.log().len(), 15 + steps_after_bundle, "{failing}");
        assert!(!runner.said().contains("✓"), "{failing}");
    }
}
