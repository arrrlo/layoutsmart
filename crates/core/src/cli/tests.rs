use super::*;
use crate::service::Backend;
use crate::store::ProfileStore;
use crate::testing::*;
use std::fs;

fn strings(args: &[&str]) -> Vec<String> {
    args.iter().map(|s| s.to_string()).collect()
}

/// Runs the CLI and returns (exit code, stdout, stderr).
fn cli(sys: &FakeSystem, store: &ProfileStore, args: &[&str]) -> (i32, String, String) {
    let clock = FixedClock(NOW);
    let backend = Backend {
        sys,
        store,
        clock: &clock,
    };
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let code = run(&strings(args), &backend, &mut out, &mut err);
    (
        code,
        String::from_utf8(out).unwrap(),
        String::from_utf8(err).unwrap(),
    )
}

fn parsed(args: &[&str]) -> Option<Command> {
    parse(&strings(args)).ok()
}

#[test]
fn parses_every_command() {
    let name = |s: &str| s.to_string();
    assert_eq!(parsed(&[]), Some(Command::Print(USAGE)));
    assert_eq!(parsed(&["help"]), Some(Command::Print(USAGE)));
    assert_eq!(parsed(&["-h"]), Some(Command::Print(USAGE)));
    assert_eq!(parsed(&["--help"]), Some(Command::Print(USAGE)));
    assert_eq!(parsed(&["--version"]), Some(Command::Print(VERSION_LINE)));
    assert_eq!(parsed(&["-V"]), Some(Command::Print(VERSION_LINE)));
    assert_eq!(parsed(&["list"]), Some(Command::List));
    assert_eq!(parsed(&["displays"]), Some(Command::Displays));
    assert_eq!(parsed(&["profiles"]), Some(Command::Profiles));
    assert_eq!(parsed(&["restore"]), Some(Command::Restore(None)));
    assert_eq!(
        parsed(&["restore", "home"]),
        Some(Command::Restore(Some(name("home"))))
    );
    assert_eq!(parsed(&["save", "home"]), Some(Command::Save(name("home"))));
    assert_eq!(
        parsed(&["delete", "home"]),
        Some(Command::Delete(name("home")))
    );
    assert_eq!(
        parsed(&["rename", "home", "house"]),
        Some(Command::Rename(name("home"), name("house")))
    );
}

#[test]
fn the_profile_name_may_come_first() {
    assert_eq!(
        parsed(&["home", "save"]),
        Some(Command::Save("home".into()))
    );
    assert_eq!(
        parsed(&["no monitors", "restore"]),
        Some(Command::Restore(Some("no monitors".into())))
    );
    // A command word in first position is never treated as a name.
    assert_eq!(
        parsed(&["save", "restore"]),
        Some(Command::Save("restore".into()))
    );
    assert_eq!(
        parsed(&["delete", "save"]),
        Some(Command::Delete("save".into()))
    );
    // Only save and restore accept the name-first form.
    assert_eq!(parsed(&["home", "delete"]), None);
    assert_eq!(parsed(&["home", "list"]), None);
}

#[test]
fn rejects_what_it_does_not_understand() {
    for args in [
        &["frobnicate"][..],
        &["save"],
        &["save", "a", "b"],
        &["list", "extra"],
        &["rename", "only-one"],
        &["delete"],
        &["a", "b", "save"],
    ] {
        assert_eq!(parsed(args), None, "{args:?}");
    }
}

#[test]
fn unknown_commands_exit_with_2_and_point_to_help() {
    let (_dir, store) = temp_store();
    let (code, out, err) = cli(&desk(), &store, &["frobnicate", "now"]);
    assert_eq!(code, 2);
    assert_eq!(out, "");
    assert_eq!(
        err,
        "layoutsmart: unrecognised command 'frobnicate now'.\nRun 'layoutsmart help' for usage.\n"
    );
}

#[test]
fn help_and_version() {
    let (_dir, store) = temp_store();
    assert_eq!(
        cli(&desk(), &store, &["help"]),
        (0, format!("{USAGE}\n"), String::new())
    );
    assert_eq!(
        cli(&desk(), &store, &["--version"]),
        (0, format!("layoutsmart {VERSION}\n"), String::new())
    );
    assert!(!VERSION.is_empty());
    for word in [
        "save",
        "restore",
        "profiles",
        "rename",
        "delete",
        "list",
        "displays",
        "help",
        "--version",
        "LAYOUTSMART_DIR",
    ] {
        assert!(USAGE.contains(word), "{word}");
    }
}

#[test]
fn save_reports_what_was_saved() {
    let (_dir, store) = temp_store();
    let (code, out, err) = cli(&desk(), &store, &["home", "save"]);
    assert_eq!((code, err.as_str()), (0, ""));
    assert_eq!(
        out,
        format!(
            "Saved 3 windows to 'home'.\nDisplays: Built-in, DELL\nFile: {}\n",
            store.path("home").display()
        )
    );
    assert!(store.exists("home"));
}

#[test]
fn save_counts_a_single_window_in_the_singular() {
    let (_dir, store) = temp_store();
    let sys = FakeSystem::new(
        vec![display("Built-in", 10, 0.0)],
        vec![window("Mail", "Inbox", 0.0)],
    );
    let (_, out, _) = cli(&sys, &store, &["save", "solo"]);
    assert!(
        out.starts_with("Saved 1 window to 'solo'.\nDisplays: Built-in\n"),
        "{out}"
    );
}

#[test]
fn failures_exit_with_1_and_explain() {
    let (_dir, store) = temp_store();
    let (code, out, err) = cli(&desk().untrusted(), &store, &["save", "home"]);
    assert_eq!((code, out.as_str()), (1, ""));
    assert!(
        err.starts_with("layoutsmart: Layoutsmart needs Accessibility permission"),
        "{err}"
    );
    assert!(err.ends_with("Accessibility.\n"), "{err}");

    let (code, _, err) = cli(&desk(), &store, &["save", "a/b"]);
    assert_eq!(code, 1);
    assert_eq!(err, "layoutsmart: Profile name can't contain '/'.\n");
}

#[test]
fn restore_lists_the_windows_it_moved() {
    let (_dir, store) = temp_store();
    cli(&desk(), &store, &["save", "home"]);

    let mut shuffled = desk();
    shuffled.windows[0] = window("Safari", "Apple", 900.0);
    shuffled.windows[1] = window("Ghostty", "", 0.0);
    shuffled.windows[2] = window("Mail", "Inbox", 0.0);
    shuffled.stuck = vec![2];
    let (code, out, err) = cli(&shuffled, &store, &["restore", "home"]);
    assert_eq!((code, err.as_str()), (0, ""));
    assert_eq!(
        out,
        "Restored 'home': moved 2 windows.\n  Safari: Apple\n  Ghostty: <untitled>\n\
         Could not move 1 window:\n  Mail: Inbox\n"
    );
}

#[test]
fn restore_with_nothing_to_do_says_so() {
    let (_dir, store) = temp_store();
    cli(&desk(), &store, &["save", "home"]);
    let (code, out, _) = cli(&desk(), &store, &["home", "restore"]);
    assert_eq!(
        (code, out.as_str()),
        (0, "Restored 'home': moved 0 windows.\n")
    );
}

#[test]
fn restore_without_a_name_picks_the_matching_profile() {
    let (_dir, store) = temp_store();
    let (code, _, err) = cli(&desk(), &store, &["restore"]);
    assert_eq!(code, 1);
    assert_eq!(
        err,
        "layoutsmart: No saved profile matches the displays that are connected now.\n"
    );

    cli(&desk(), &store, &["save", "home"]);
    let (code, out, _) = cli(&desk(), &store, &["restore"]);
    assert_eq!(
        (code, out.as_str()),
        (0, "Restored 'home': moved 0 windows.\n")
    );

    let (code, _, err) = cli(&desk(), &store, &["restore", "nope"]);
    assert_eq!(code, 1);
    assert_eq!(
        err,
        "layoutsmart: There is no saved profile named 'nope'.\n"
    );
}

#[test]
fn list_shows_one_aligned_line_per_window() {
    let (_dir, store) = temp_store();
    let (code, out, _) = cli(&desk(), &store, &["list"]);
    assert_eq!(code, 0);
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines.len(), 3);
    let words: Vec<&str> = lines[1].split_whitespace().collect();
    assert_eq!(words, ["Ghostty", "2000,50", "800x600", "zsh"]);
    assert_eq!(lines[0].find("Apple"), lines[1].find("zsh"));
    assert_eq!(
        lines[0].find("100,50").map(|i| i + 3),
        lines[1].find("2000,50").map(|i| i + 4)
    );
    assert!(out.ends_with("Inbox\n"));

    let (code, out, err) = cli(&desk().untrusted(), &store, &["list"]);
    assert_eq!((code, out.as_str()), (1, ""));
    assert!(err.contains("Accessibility"));
}

#[test]
fn displays_shows_each_display_and_the_matching_profile() {
    let (_dir, store) = temp_store();
    let (code, out, _) = cli(&desk(), &store, &["displays"]);
    assert_eq!(code, 0);
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines.len(), 3);
    let words: Vec<&str> = lines[1].split_whitespace().collect();
    assert_eq!(
        words,
        ["DELL", "id=1-2-20", "origin=1920,0", "size=1920x1080"]
    );
    assert_eq!(lines[0].find("id="), lines[1].find("id="));
    assert_eq!(lines[0].find("origin="), lines[1].find("origin="));
    assert_eq!(lines[2], "Matching profile: none");

    cli(&desk(), &store, &["save", "home"]);
    let (_, out, _) = cli(&desk(), &store, &["displays"]);
    assert!(out.ends_with("\nMatching profile: home\n"), "{out}");
}

#[test]
fn displays_works_without_accessibility_permission() {
    let (_dir, store) = temp_store();
    let (code, out, _) = cli(&desk().untrusted(), &store, &["displays"]);
    assert_eq!(code, 0);
    assert!(out.contains("Built-in"));
}

#[test]
fn profiles_lists_saved_profiles_and_marks_the_matching_ones() {
    let (_dir, store) = temp_store();
    let (code, out, _) = cli(&desk(), &store, &["profiles"]);
    assert_eq!(code, 0);
    assert_eq!(
        out,
        "No saved profiles. Create one with: layoutsmart <name> save\n"
    );

    cli(&desk(), &store, &["save", "home"]);
    let laptop = FakeSystem::new(
        vec![display("Built-in", 10, 0.0)],
        vec![window("Mail", "Inbox", 0.0)],
    );
    cli(&laptop, &store, &["save", "travel"]);
    fs::write(store.path("broken"), "{").unwrap();

    let (code, out, _) = cli(&desk(), &store, &["profiles"]);
    assert_eq!(code, 0);
    assert_eq!(
        out,
        "! broken  (unreadable file)\n\
         * home  3 windows  saved 2026-11-11 09:42 UTC  [Built-in, DELL]\n\
         \x20 travel  1 window  saved 2026-11-11 09:42 UTC  [Built-in]\n"
    );
}

#[test]
fn rename_and_delete() {
    let (_dir, store) = temp_store();
    cli(&desk(), &store, &["save", "home"]);

    let (code, out, _) = cli(&desk(), &store, &["rename", "home", "house"]);
    assert_eq!((code, out.as_str()), (0, "Renamed 'home' to 'house'.\n"));
    assert_eq!(store.names(), ["house"]);

    let (code, _, err) = cli(&desk(), &store, &["rename", "house", "list"]);
    assert_eq!(code, 1);
    assert_eq!(
        err,
        "layoutsmart: 'list' is a command and can't be a profile name.\n"
    );

    let (code, out, _) = cli(&desk(), &store, &["delete", "house"]);
    assert_eq!((code, out.as_str()), (0, "Deleted 'house'.\n"));
    assert!(store.names().is_empty());

    let (code, _, err) = cli(&desk(), &store, &["delete", "house"]);
    assert_eq!(code, 1);
    assert_eq!(
        err,
        "layoutsmart: There is no saved profile named 'house'.\n"
    );
}

#[test]
fn types_are_cloneable_and_printable() {
    let c = Command::Save("x".into());
    assert_eq!(c.clone(), c);
    assert_eq!(format!("{c:?}"), "Save(\"x\")");
    let failure = parse(&strings(&["nope"])).unwrap_err();
    assert_eq!(failure.clone(), failure);
    assert!(format!("{failure:?}").starts_with("Failure"));
    assert_eq!(failure.code, 2);
}
