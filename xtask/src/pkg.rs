//! Builds the installer: `dist/Layoutsmart-<version>.pkg`.
//!
//! One file that installs everything: the app into /Applications (always there, never
//! over some other copy on the disk), the `layoutsmart` command, and it starts the app.

use crate::bundle::{APP, BUNDLE_ID, MIN_MACOS, Signing, VERSION, bundle, render};
use crate::cmd::Cmd;
use crate::runner::{Runner, run_all};

const STAGING: &str = "dist/pkgroot";
const COMPONENT: &str = "dist/component.pkg";
const DISTRIBUTION: &str = "dist/distribution.xml";
/// Where the finished installer is unpacked to check what it really contains.
const CHECK: &str = "dist/pkgcheck";

pub fn installer_path(version: &str) -> String {
    format!("dist/Layoutsmart-{version}.pkg")
}

/// The installer's script: title, minimum macOS, supported processors, welcome and
/// closing screens.
pub fn distribution(version: &str) -> String {
    render(
        include_str!("../../packaging/distribution.xml.in"),
        &[
            ("@VERSION@", version),
            ("@BUNDLE_ID@", BUNDLE_ID),
            ("@MIN_MACOS@", MIN_MACOS),
        ],
    )
}

fn component_cmds() -> Vec<Cmd> {
    vec![
        Cmd::parse("rm -rf").arg(STAGING),
        Cmd::parse("mkdir -p").arg(STAGING),
        Cmd::parse("ditto")
            .arg(APP)
            .arg(&format!("{STAGING}/Layoutsmart.app")),
        Cmd::parse("pkgbuild --root")
            .arg(STAGING)
            .arg("--component-plist")
            .arg("packaging/component.plist")
            .arg("--identifier")
            .arg(BUNDLE_ID)
            .arg("--version")
            .arg(VERSION)
            .arg("--install-location")
            .arg("/Applications")
            .arg("--scripts")
            .arg("packaging/scripts")
            .arg(COMPONENT),
    ]
}

fn product_cmds(installer: &str) -> Vec<Cmd> {
    vec![
        Cmd::parse("productbuild --distribution")
            .arg(DISTRIBUTION)
            .arg("--package-path")
            .arg("dist")
            .arg("--resources")
            .arg("packaging/resources")
            .arg(installer),
        // Unpack the finished installer the way Installer will, and make sure the app
        // inside still has a valid signature.
        Cmd::parse("rm -rf").arg(CHECK),
        Cmd::parse("pkgutil --expand-full")
            .arg(installer)
            .arg(CHECK),
        Cmd::parse("codesign --verify --deep --strict")
            .arg(&format!("{CHECK}/component.pkg/Payload/Layoutsmart.app")),
        Cmd::parse("rm -rf")
            .arg(STAGING)
            .arg(COMPONENT)
            .arg(DISTRIBUTION)
            .arg(CHECK),
    ]
}

pub fn pkg(runner: &dyn Runner, signing: &Signing) -> bool {
    let installer = installer_path(VERSION);
    let built = bundle(runner, signing)
        && run_all(runner, &component_cmds())
        && runner.write(DISTRIBUTION, &distribution(VERSION))
        && run_all(runner, &product_cmds(&installer));
    if built {
        runner.say(&format!("✓ {installer}"));
    }
    built
}

#[cfg(test)]
mod tests;
