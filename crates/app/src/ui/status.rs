//! The menu bar icon and its menu.

use super::state::App;
use layoutsmart_core::menu::{Menu, MenuEntry};
use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2::{MainThreadOnly, sel};
use objc2_app_kit::{NSImage, NSMenu, NSMenuItem, NSStatusBar, NSVariableStatusItemLength};
use objc2_foundation::{NSString, ns_string};
use std::rc::Rc;

/// SF Symbol shown in the menu bar (drawn as a template, so it follows light/dark mode).
const ICON: &str = "macwindow.on.rectangle";

pub fn install(app: &'static App) {
    let item = NSStatusBar::systemStatusBar().statusItemWithLength(NSVariableStatusItemLength);
    let icon = NSImage::imageWithSystemSymbolName_accessibilityDescription(
        &NSString::from_str(ICON),
        Some(ns_string!("Layoutsmart")),
    );
    item.button(app.mtm)
        .iter()
        .for_each(|button| button.setImage(icon.as_deref()));

    let menu = NSMenu::new(app.mtm);
    // Items are enabled or disabled by the core, not guessed by AppKit.
    menu.setAutoenablesItems(false);
    menu.setDelegate(Some(ProtocolObject::from_ref(&*app.controller)));
    item.setMenu(Some(&menu));
    app.status_item.replace(Some(item));
}

fn item_for(app: &App, entry: &MenuEntry, index: usize) -> Retained<NSMenuItem> {
    if entry.is_separator() {
        return NSMenuItem::separatorItem(app.mtm);
    }
    // SAFETY: `menuItemClicked:` is implemented by the controller and takes the sender.
    let item = unsafe {
        let item = NSMenuItem::initWithTitle_action_keyEquivalent(
            NSMenuItem::alloc(app.mtm),
            &NSString::from_str(&entry.title()),
            Some(sel!(menuItemClicked:)),
            ns_string!(""),
        );
        item.setTarget(Some(&app.controller));
        item
    };
    item.setTag(index as isize);
    item.setEnabled(entry.is_enabled());
    item
}

/// Called each time the menu is about to open: profiles are read only then.
pub fn rebuild(app: &App, menu: &NSMenu) {
    let model = Rc::new(Menu::load(&app.store));
    menu.removeAllItems();
    model
        .entries()
        .iter()
        .enumerate()
        .for_each(|(index, entry)| menu.addItem(&item_for(app, entry, index)));
    app.menu.replace(model);
}

pub fn clicked(app: &App, tag: isize) {
    // Not borrowed while the click runs: a dialog may be open for a long time, and the
    // menu can be reopened (and rebuilt) meanwhile.
    let menu = Rc::clone(&app.menu.borrow());
    menu.click(
        usize::try_from(tag).unwrap_or(usize::MAX),
        &app.session(),
        app,
    );
}
