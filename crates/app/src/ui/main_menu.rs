//! A menu bar app shows no main menu, but AppKit still routes ⌘C, ⌘V and friends
//! through one. Without this, copy and paste would not work in the name box.

use objc2::runtime::Sel;
use objc2::{MainThreadMarker, MainThreadOnly, sel};
use objc2_app_kit::{NSApplication, NSMenu, NSMenuItem};
use objc2_foundation::{NSString, ns_string};

fn item(
    mtm: MainThreadMarker,
    title: &str,
    action: Sel,
    key: &str,
) -> objc2::rc::Retained<NSMenuItem> {
    // SAFETY: the actions are standard responder-chain selectors taking one sender.
    unsafe {
        NSMenuItem::initWithTitle_action_keyEquivalent(
            NSMenuItem::alloc(mtm),
            &NSString::from_str(title),
            Some(action),
            &NSString::from_str(key),
        )
    }
}

pub fn install(mtm: MainThreadMarker) {
    let edit = NSMenu::initWithTitle(NSMenu::alloc(mtm), ns_string!("Edit"));
    [
        ("Undo", sel!(undo:), "z"),
        ("Cut", sel!(cut:), "x"),
        ("Copy", sel!(copy:), "c"),
        ("Paste", sel!(paste:), "v"),
        ("Select All", sel!(selectAll:), "a"),
        ("Close Window", sel!(performClose:), "w"),
    ]
    .iter()
    .for_each(|(title, action, key)| edit.addItem(&item(mtm, title, *action, key)));

    let holder = NSMenuItem::new(mtm);
    holder.setSubmenu(Some(&edit));
    let main = NSMenu::new(mtm);
    main.addItem(&holder);
    NSApplication::sharedApplication(mtm).setMainMenu(Some(&main));
}
