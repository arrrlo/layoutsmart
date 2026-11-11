<p align="center">
  <img src="docs/icon.png" width="128" height="128" alt="Layoutsmart icon">
</p>

<h1 align="center">Layoutsmart</h1>

<p align="center"><em>Outsmart your desktop.</em></p>

<p align="center">
  <a href="https://github.com/arrrlo/layoutsmart/actions/workflows/ci.yml"><img src="https://github.com/arrrlo/layoutsmart/actions/workflows/ci.yml/badge.svg?branch=main" alt="CI"></a>
  <a href="https://github.com/arrrlo/layoutsmart/releases/latest"><img src="https://img.shields.io/github/v/release/arrrlo/layoutsmart?label=release" alt="Latest release"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue.svg" alt="MIT licence"></a>
  <img src="https://img.shields.io/badge/macOS-13%2B-black?logo=apple" alt="macOS 13 or later">
  <img src="https://img.shields.io/badge/Intel%20%7C%20Apple%20Silicon-universal-8A2BE2" alt="Intel and Apple Silicon">
</p>

Layoutsmart is a small macOS menu bar app that remembers where your windows are and puts
them back with one click. Plug in a different set of displays, watch macOS scatter
everything, then pick "home" or "office" from the menu and carry on.

![The Layoutsmart dashboard showing a profile with three displays, a row of desktop tabs for each, and the windows on the selected desktops](docs/dashboard.png)

## What it does

- **Save Window Positions…** stores where every window is — its position and size, the
  screen and desktop (Space) it is on, and whether it is full screen — under a name you
  choose: "home", "office", "no monitors", anything.
- **Click a profile** in the menu to put the windows back: on their screen and desktop,
  at their size, in or out of full screen.
- **Dashboard** lists your profiles and draws each one: the displays, how they are
  arranged, and the windows on each desktop. Rename or delete profiles there.
- **`layoutsmart` command** does the same from Terminal, for scripts and shortcuts.
- Starts when you log in and sits in the menu bar. No Dock icon, no window unless you
  ask for one.

It is deliberately manual: nothing moves until you click.

> **Desktops and full screen are new on `main`** and will be version 0.2. The released
> 0.1.0 saves and restores only the windows on the desktops that are in view, and leaves
> full-screen windows alone.

## Install

1. Download `Layoutsmart-<version>.pkg` from the
   [latest release](https://github.com/arrrlo/layoutsmart/releases/latest).
2. Double-click it. Because Layoutsmart is not signed with a paid Apple developer
   certificate, macOS will refuse the first time. Click **Done**, then open
   **System Settings → Privacy & Security**, scroll down and click **Open Anyway**.
3. Follow the installer. It puts Layoutsmart in Applications, adds the `layoutsmart`
   command, and starts the app. Nothing else needs to be installed.
4. macOS asks for **Accessibility** permission, which is what lets an app see and move
   other apps' windows. Switch on Layoutsmart under
   **System Settings → Privacy & Security → Accessibility**.

Prefer Terminal? This installs the same package without the Gatekeeper detour:

```sh
sudo installer -pkg ~/Downloads/Layoutsmart-*.pkg -target /
```

Requires macOS 13 or later, on Intel or Apple Silicon. Before a build is released it is
installed and exercised automatically on macOS 15 and 26, on both kinds of Mac.

### Uninstall

```sh
pkill -x Layoutsmart
sudo rm -rf /Applications/Layoutsmart.app /usr/local/bin/layoutsmart
rm -rf ~/.config/layoutsmart        # your saved profiles
```

Then remove Layoutsmart under System Settings → General → Login Items, and under
Privacy & Security → Accessibility.

## Using it

Click the icon in the menu bar:

| Menu item | What happens |
|---|---|
| **Save Window Positions…** | Asks for a name and saves the current layout. Using an existing name asks before replacing it. |
| *your profiles* | Click one to restore it. |
| **Open Dashboard…** | Shows every profile with a picture of its displays and windows, with a tab for each desktop and full-screen app of each screen. Rename and Delete are here. |
| **Quit Layoutsmart** | Stops the app until you open it again or log in. |

### Command line

```
layoutsmart <name> save          Save the current window positions as <name>
layoutsmart <name> restore       Put the windows back where <name> has them
layoutsmart restore              Restore the profile saved with the displays connected now
layoutsmart profiles             List saved profiles (* = saved with the current displays)
layoutsmart rename <old> <new>   Rename a profile
layoutsmart delete <name>        Delete a profile
layoutsmart list                 Show open windows: position, size, screen and desktop
layoutsmart displays             Show connected displays and each one's desktops
layoutsmart help | --version
```

The terminal app you run it from needs Accessibility permission too.

### Good to know

- Profiles are plain JSON files in `~/.config/layoutsmart/` (set `LAYOUTSMART_DIR` to use
  another folder). The file name is the profile name.
- Each profile remembers which displays it was saved with, so `layoutsmart restore`
  without a name can pick the right one.
- Windows are matched by app, then by title, preferring a window that is still on the
  desktop it was saved on. Windows whose title has changed (a browser showing a different
  page) are matched in the order they were opened, first among the windows on the same
  desktop.
- Minimised windows are left alone, and so are apps that are not running.
- If everything is where the profile has it, restoring does nothing at all.
- Layoutsmart never connects to the internet and collects nothing.

### Desktops and full screen

macOS gives an app no direct way to move another app's window to a different desktop, or
to make a screen show a different desktop. So Layoutsmart does what you would do by hand,
only faster:

- **To put a window on a desktop**, it brings that desktop into view on its screen and
  moves the window so that it lands there (off to another screen and straight back; with
  a single screen, into full screen and out again).
- **To bring a desktop into view**, it presses the Mission Control shortcuts "Move left a
  space" and "Move right a space" (Control–← and Control–→) with the pointer on that
  screen, then puts the pointer back. If you have switched those shortcuts off or changed
  them, it presses nothing, leaves windows on the desktop they are on, and tells you.
- **Full screen** is switched on or off for each window that needs it. A new full-screen
  desktop always appears at the end of its screen's row, so Layoutsmart makes them in the
  order they were saved in.

What that means in practice:

- You will see screens slide between desktops while a restore runs, and each window that
  enters or leaves full screen takes macOS about a second. The menu does not respond
  until it is done. Windows that only need to move or resize are instant, on any desktop.
- When it has finished, each screen shows what it showed when the profile was saved, and
  the app you were using is in front again — unless its windows are now all on desktops
  that are out of view: bringing it forward would make macOS switch to one of those.
- macOS only lets other apps reach a window once its desktop has been in view since the
  window's app started. A window that has not been is saved without its title, and on
  restore its desktop is visited once — only if the window has to move.
- Layoutsmart cannot add desktops. If a screen has fewer than when the profile was saved,
  the windows of the missing ones go to its last desktop.
- Full-screen desktops that already exist are not put in a different order, and two
  windows sharing one full-screen desktop (Split View) come back as two.
- With a single screen, a window of an app that has no full-screen mode cannot change
  desktop; it is moved and resized where it is, and reported.
- Seeing other desktops relies on parts of macOS that Apple does not document. They are
  checked on every macOS version the installer is tested on; where they are missing,
  Layoutsmart handles the windows on the desktops in view, as version 0.1 did.

## Fast and light

Measured on an Apple Silicon MacBook Pro with three displays:

| | |
|---|---|
| Reading every window on every desktop (18 windows, 11 desktops) | about 90 ms |
| Memory while idle | about 20 MB (about 32 MB once the dashboard has been opened) |
| CPU while idle | 0% — no timers, no polling |
| Size on disk | 2.4 MB for the app, 1.3 MB for the installer |

Asking another app about its windows takes macOS 10–25 ms per app, so Layoutsmart asks
all apps at the same time, and reads all of a window's properties in one request. A
restore that only moves and resizes windows takes about a tenth of a second in all, most
of it that reading; switching desktops and full screen takes as long as macOS's
animations do.

## Building from source

You need a Mac with Rust (the exact version is pinned in `rust-toolchain.toml`) and
Apple's command line tools.

```sh
cargo xtask bundle       # dist/Layoutsmart.app
cargo xtask pkg          # dist/Layoutsmart-<version>.pkg
cargo xtask help         # everything else
```

The layout of the code:

| Folder | What it holds |
|---|---|
| `crates/core` | Every decision: matching windows, the steps of a restore, profiles on disk, the dialogs' logic, the menu, the dashboard picture, the command line. Pure Rust, no `unsafe`. |
| `crates/macos` | The calls into macOS: Accessibility, displays, desktops, running apps, the pointer and keyboard. |
| `crates/app` | The two programs and the AppKit windows. |
| `xtask` | Build tasks: quality gates, app bundle, installer. |

## Contributing

Layoutsmart is built test-first, and the rules are enforced by `cargo xtask ci`, which is
also what CI runs:

| Gate | Requirement |
|---|---|
| Format and lints | `cargo fmt`, `cargo clippy` with warnings as errors |
| Tests | Pass as Apple Silicon and as Intel builds |
| Coverage | 100% of lines, regions and functions ([cargo-llvm-cov](https://github.com/taiki-e/cargo-llvm-cov)) |
| CRAP | No function above 10 ([cargo-crap](https://github.com/minikin/cargo-crap)) |
| Mutation testing | No surviving mutant ([cargo-mutants](https://mutants.rs)) |
| `lint-shell` | See below |

One-time setup:

```sh
cargo install cargo-llvm-cov@0.9.1 cargo-mutants@27.1.0 cargo-crap@0.9.0 --locked
```

**The one exception.** Code that only talks to macOS (`crates/macos`, `crates/app/src`
and the process launcher in `xtask/src/sh.rs`) cannot be unit-tested without driving a
real desktop, so it is left out of the coverage and mutation figures. In exchange it is
not allowed to decide anything: the CRAP gate scores it as 0% covered, which limits each
function to a single branch, and `lint-shell` forbids any branch inside the AppKit
callback methods. If you find yourself wanting an `if` there, the logic belongs in
`crates/core`, with a test.

That code is checked by `cargo xtask smoke`, which drives the real app through its menu
bar icon (it needs Accessibility permission for your terminal). CI installs the finished
package on four kinds of Mac and runs the same script there.

Working method: write the test, watch it fail, write the least code that makes it pass,
tidy up, run `cargo xtask check`. Run `cargo xtask mutants` before you commit.

### Releasing

1. Set the new version in `Cargo.toml` (`[workspace.package]`) and commit.
2. Tag it `v<version>` and push the tag.

CI then runs every gate, builds the installer, installs and starts it on each supported
macOS version, and publishes a GitHub release with the `.pkg` and its checksum.

## Licence

[MIT](LICENSE)
