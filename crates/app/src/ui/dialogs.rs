//! The dialogs the core asks for. What they say and what happens next is the core's
//! business; this only puts them on screen.

use layoutsmart_core::ports::{Question, TextPrompt, Ui};
use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2_app_kit::{NSAlert, NSAlertFirstButtonReturn, NSApplication, NSTextField, NSWorkspace};
use objc2_foundation::{NSPoint, NSRect, NSSize, NSString, NSURL, ns_string};

const ACCESSIBILITY_SETTINGS: &str =
    "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility";

pub struct Dialogs {
    mtm: MainThreadMarker,
}

impl Dialogs {
    pub fn new(mtm: MainThreadMarker) -> Dialogs {
        Dialogs { mtm }
    }

    /// An alert in front of everything (a menu bar app is never "active" on its own).
    fn alert_with(&self, title: &str, message: &str) -> Retained<NSAlert> {
        #[allow(deprecated)] // `activate()` needs macOS 14; this also covers macOS 13.
        NSApplication::sharedApplication(self.mtm).activateIgnoringOtherApps(true);
        let alert = NSAlert::new(self.mtm);
        alert.setMessageText(&NSString::from_str(title));
        alert.setInformativeText(&NSString::from_str(message));
        alert
    }
}

impl Ui for Dialogs {
    fn ask_text(&self, prompt: &TextPrompt) -> Option<String> {
        let alert = self.alert_with(&prompt.title, &prompt.message);
        alert.addButtonWithTitle(&NSString::from_str(&prompt.confirm));
        alert.addButtonWithTitle(ns_string!("Cancel"));
        let field = NSTextField::textFieldWithString(&NSString::from_str(&prompt.value), self.mtm);
        field.setFrame(NSRect::new(
            NSPoint::new(0.0, 0.0),
            NSSize::new(280.0, 24.0),
        ));
        alert.setAccessoryView(Some(&field));
        alert.window().setInitialFirstResponder(Some(&field));
        (alert.runModal() == NSAlertFirstButtonReturn).then(|| field.stringValue().to_string())
    }

    fn confirm(&self, question: &Question) -> bool {
        let alert = self.alert_with(&question.title, &question.message);
        alert
            .addButtonWithTitle(&NSString::from_str(&question.confirm))
            .setHasDestructiveAction(question.destructive);
        alert.addButtonWithTitle(ns_string!("Cancel"));
        alert.runModal() == NSAlertFirstButtonReturn
    }

    fn alert(&self, title: &str, message: &str) {
        let alert = self.alert_with(title, message);
        alert.addButtonWithTitle(ns_string!("OK"));
        alert.runModal();
    }

    fn open_accessibility_settings(&self) {
        NSURL::URLWithString(&NSString::from_str(ACCESSIBILITY_SETTINGS))
            .map(|url| NSWorkspace::sharedWorkspace().openURL(&url));
    }
}
