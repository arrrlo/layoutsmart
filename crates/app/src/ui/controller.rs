//! The Objective-C objects AppKit calls back into. Every method is a single call into
//! an ordinary function, so that all behaviour lives where it can be read and measured.

use super::state::app;
use super::{dashboard, launch, paint, status};
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{MainThreadMarker, MainThreadOnly, define_class, msg_send};
use objc2_app_kit::{
    NSApplicationDelegate, NSControlTextEditingDelegate, NSMenu, NSMenuDelegate, NSMenuItem,
    NSTableColumn, NSTableView, NSTableViewDataSource, NSTableViewDelegate, NSView,
    NSWindowDelegate,
};
use objc2_foundation::{NSInteger, NSNotification, NSObject, NSObjectProtocol, NSRect};

define_class!(
    // SAFETY: NSObject has no subclassing requirements, and Controller has no Drop.
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "LayoutsmartController"]
    pub struct Controller;

    impl Controller {
        #[unsafe(method(menuItemClicked:))]
        fn menu_item_clicked(&self, sender: &NSMenuItem) {
            status::clicked(app(), sender.tag())
        }

        #[unsafe(method(renameClicked:))]
        fn rename_clicked(&self, _sender: Option<&AnyObject>) {
            dashboard::rename(app())
        }

        #[unsafe(method(deleteClicked:))]
        fn delete_clicked(&self, _sender: Option<&AnyObject>) {
            dashboard::delete(app())
        }
    }

    unsafe impl NSObjectProtocol for Controller {}

    unsafe impl NSApplicationDelegate for Controller {
        #[unsafe(method(applicationDidFinishLaunching:))]
        fn did_finish_launching(&self, _notification: &NSNotification) {
            launch::finished(app())
        }
    }

    unsafe impl NSMenuDelegate for Controller {
        #[unsafe(method(menuNeedsUpdate:))]
        fn menu_needs_update(&self, menu: &NSMenu) {
            status::rebuild(app(), menu)
        }
    }

    unsafe impl NSWindowDelegate for Controller {
        #[unsafe(method(windowWillClose:))]
        fn window_will_close(&self, _notification: &NSNotification) {
            dashboard::closed(app())
        }
    }

    unsafe impl NSTableViewDataSource for Controller {
        #[unsafe(method(numberOfRowsInTableView:))]
        fn number_of_rows(&self, _table: &NSTableView) -> NSInteger {
            dashboard::row_count(app())
        }

        #[unsafe(method_id(tableView:objectValueForTableColumn:row:))]
        fn object_value(
            &self,
            _table: &NSTableView,
            _column: Option<&NSTableColumn>,
            row: NSInteger,
        ) -> Option<Retained<AnyObject>> {
            dashboard::row_value(app(), row)
        }
    }

    unsafe impl NSControlTextEditingDelegate for Controller {}

    unsafe impl NSTableViewDelegate for Controller {
        #[unsafe(method(tableViewSelectionDidChange:))]
        fn selection_changed(&self, _notification: &NSNotification) {
            dashboard::selection_changed(app())
        }
    }
);

impl Controller {
    pub fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(());
        // SAFETY: plain NSObject initialisation.
        unsafe { msg_send![super(this), init] }
    }
}

define_class!(
    // SAFETY: NSView may be subclassed to draw; PreviewView has no Drop.
    #[unsafe(super(NSView))]
    #[thread_kind = MainThreadOnly]
    #[name = "LayoutsmartPreview"]
    pub struct PreviewView;

    impl PreviewView {
        #[unsafe(method(drawRect:))]
        fn draw_rect(&self, _dirty: NSRect) {
            paint::preview(app(), self.bounds())
        }

        /// Top-left origin, like the screen coordinates being drawn.
        #[unsafe(method(isFlipped))]
        fn is_flipped(&self) -> bool {
            true
        }
    }
);

impl PreviewView {
    pub fn new(mtm: MainThreadMarker, frame: NSRect) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(());
        // SAFETY: NSView's designated initialiser.
        unsafe { msg_send![super(this), initWithFrame: frame] }
    }
}
