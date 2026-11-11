//! The dashboard window: profile list on the left, picture of the selected profile on
//! the right. All state is in `layoutsmart_core::dashboard::Dashboard`.

use super::controller::PreviewView;
use super::state::App;
use layoutsmart_core::dashboard::{Dashboard, TabRow};
use layoutsmart_core::ports::Host;
use objc2::AnyThread;
use objc2::rc::Retained;
use objc2::runtime::{AnyObject, ProtocolObject, Sel};
use objc2::{MainThreadOnly, sel};
use objc2_app_kit::{
    NSApplication, NSAutoresizingMaskOptions, NSBackingStoreType, NSBitmapImageFileType,
    NSBitmapImageRep, NSButton, NSDeviceRGBColorSpace, NSFont, NSScrollView, NSSegmentDistribution,
    NSSegmentSwitchTracking, NSSegmentedControl, NSTableColumn, NSTableView, NSTextField, NSView,
    NSWindow, NSWindowStyleMask,
};
use objc2_foundation::{
    NSArray, NSData, NSDate, NSDictionary, NSIndexSet, NSPoint, NSRect, NSRunLoop, NSSize,
    NSString, ns_string,
};

const WIDTH: f64 = 900.0;
const HEIGHT: f64 = 560.0;
const SIDEBAR: f64 = 220.0;
/// Height of the strip holding the buttons (left) and the summary line (right).
const BAR: f64 = 44.0;
/// Height of one screen's row of desktop tabs, between the picture and that strip.
const TAB_ROW: f64 = 30.0;
/// Room for the screen's name in front of its tabs.
const TAB_NAME: f64 = 150.0;
/// The widest a single tab gets.
const TAB_WIDTH: f64 = 110.0;

pub struct DashboardUi {
    window: Retained<NSWindow>,
    table: Retained<NSTableView>,
    preview: Retained<PreviewView>,
    /// Holds a row of tabs for each screen of the selected profile.
    tabs: Retained<NSView>,
    summary: Retained<NSTextField>,
    rename: Retained<NSButton>,
    delete: Retained<NSButton>,
}

fn rect(x: f64, y: f64, width: f64, height: f64) -> NSRect {
    NSRect::new(NSPoint::new(x, y), NSSize::new(width, height))
}

fn button(app: &App, title: &str, action: Sel, x: f64) -> Retained<NSButton> {
    // SAFETY: the controller implements both actions; each takes the sender.
    let button = unsafe {
        NSButton::buttonWithTitle_target_action(
            &NSString::from_str(title),
            Some(&app.controller),
            Some(action),
            app.mtm,
        )
    };
    button.setFrame(rect(x, 8.0, 96.0, 28.0));
    button
}

fn table(app: &App) -> Retained<NSTableView> {
    let table = NSTableView::initWithFrame(
        NSTableView::alloc(app.mtm),
        rect(0.0, 0.0, SIDEBAR, HEIGHT - BAR),
    );
    let column =
        NSTableColumn::initWithIdentifier(NSTableColumn::alloc(app.mtm), ns_string!("profile"));
    column.setWidth(SIDEBAR - 24.0);
    column.setEditable(false);
    table.addTableColumn(&column);
    table.setHeaderView(None);
    table.setRowHeight(26.0);
    // SAFETY: the controller implements both protocols and lives as long as the app.
    unsafe {
        table.setDataSource(Some(ProtocolObject::from_ref(&*app.controller)));
        table.setDelegate(Some(ProtocolObject::from_ref(&*app.controller)));
    }
    table
}

fn window(app: &App) -> Retained<NSWindow> {
    let style = NSWindowStyleMask::Titled
        | NSWindowStyleMask::Closable
        | NSWindowStyleMask::Miniaturizable
        | NSWindowStyleMask::Resizable;
    // SAFETY: standard window creation; the window is released by us, not on close.
    let window = unsafe {
        let window = NSWindow::initWithContentRect_styleMask_backing_defer(
            NSWindow::alloc(app.mtm),
            rect(0.0, 0.0, WIDTH, HEIGHT),
            style,
            NSBackingStoreType::Buffered,
            false,
        );
        window.setReleasedWhenClosed(false);
        window
    };
    window.setTitle(ns_string!("Layoutsmart"));
    window.setContentMinSize(NSSize::new(640.0, 380.0));
    window.setDelegate(Some(ProtocolObject::from_ref(&*app.controller)));
    window.center();
    window
}

fn build(app: &App) -> DashboardUi {
    let window = window(app);
    let table = table(app);
    let scroll = NSScrollView::initWithFrame(
        NSScrollView::alloc(app.mtm),
        rect(0.0, BAR, SIDEBAR, HEIGHT - BAR),
    );
    scroll.setDocumentView(Some(&table));
    scroll.setHasVerticalScroller(true);
    scroll.setAutoresizingMask(NSAutoresizingMaskOptions::ViewHeightSizable);

    let rename = button(app, "Rename…", sel!(renameClicked:), 10.0);
    let delete = button(app, "Delete…", sel!(deleteClicked:), 112.0);

    let summary = NSTextField::labelWithString(ns_string!(""), app.mtm);
    summary.setFrame(rect(SIDEBAR + 16.0, 12.0, WIDTH - SIDEBAR - 32.0, 20.0));
    summary.setAutoresizingMask(NSAutoresizingMaskOptions::ViewWidthSizable);

    let preview = PreviewView::new(app.mtm, rect(SIDEBAR, BAR, WIDTH - SIDEBAR, HEIGHT - BAR));
    preview.setAutoresizingMask(
        NSAutoresizingMaskOptions::ViewWidthSizable | NSAutoresizingMaskOptions::ViewHeightSizable,
    );

    let tabs = NSView::initWithFrame(
        NSView::alloc(app.mtm),
        rect(SIDEBAR, BAR, WIDTH - SIDEBAR, 0.0),
    );
    tabs.setAutoresizingMask(NSAutoresizingMaskOptions::ViewWidthSizable);

    let content = window
        .contentView()
        .expect("a new window has a content view");
    content.addSubview(&tabs);
    content.addSubview(&scroll);
    content.addSubview(&rename);
    content.addSubview(&delete);
    content.addSubview(&summary);
    content.addSubview(&preview);
    DashboardUi {
        window,
        table,
        preview,
        tabs,
        summary,
        rename,
        delete,
    }
}

/// Runs `action` with the dashboard's views, if the window is open.
fn with_ui(app: &App, action: impl Fn(&App, &DashboardUi)) {
    app.dashboard.borrow().iter().for_each(|ui| action(app, ui));
}

/// One screen's row: its name, then a tab for each desktop and each full-screen app.
/// Rows are stacked with the first screen on top; `below` is how many rows follow it.
fn add_tab_row(app: &App, ui: &DashboardUi, index: usize, below: usize, row: &TabRow) {
    let y = below as f64 * TAB_ROW + 3.0;
    let name = NSTextField::labelWithString(&NSString::from_str(&row.name), app.mtm);
    name.setFont(Some(&NSFont::systemFontOfSize(11.0)));
    name.setFrame(rect(16.0, y + 4.0, TAB_NAME - 8.0, 16.0));

    let labels: Vec<Retained<NSString>> = row
        .tabs
        .iter()
        .map(|label| NSString::from_str(label))
        .collect();
    // SAFETY: the controller implements `tabChosen:`, which takes the control.
    let control = unsafe {
        NSSegmentedControl::segmentedControlWithLabels_trackingMode_target_action(
            &NSArray::from_retained_slice(&labels),
            NSSegmentSwitchTracking::SelectOne,
            Some(&app.controller),
            Some(sel!(tabChosen:)),
            app.mtm,
        )
    };
    let room = ui.tabs.bounds().size.width - TAB_NAME - 32.0;
    let width = room.min(row.tabs.len() as f64 * TAB_WIDTH);
    control.setSegmentDistribution(NSSegmentDistribution::FillEqually);
    control.setFrame(rect(16.0 + TAB_NAME, y, width, 24.0));
    control.setSelectedSegment(row.selected as isize);
    control.setTag(index as isize);
    ui.tabs.addSubview(&name);
    ui.tabs.addSubview(&control);
}

/// Gives the rows of tabs the height they need and the picture the rest.
fn make_room(ui: &DashboardUi, content: &NSView, rows: usize) {
    let size = content.bounds().size;
    let height = rows as f64 * TAB_ROW;
    let width = size.width - SIDEBAR;
    ui.tabs.setFrame(rect(SIDEBAR, BAR, width, height));
    ui.preview.setFrame(rect(
        SIDEBAR,
        BAR + height,
        width,
        size.height - BAR - height,
    ));
}

/// Rebuilds the rows of desktop tabs for the selected profile.
fn show_tabs(app: &App, ui: &DashboardUi) {
    let rows = app.model.borrow().tab_rows(&app.store);
    ui.tabs
        .subviews()
        .iter()
        .for_each(|view| view.removeFromSuperview());
    ui.window
        .contentView()
        .iter()
        .for_each(|content| make_room(ui, content, rows.len()));
    rows.iter()
        .enumerate()
        .for_each(|(index, row)| add_tab_row(app, ui, index, rows.len() - 1 - index, row));
}

/// Summary line, buttons, tabs and picture for the current selection.
fn show_details(app: &App, ui: &DashboardUi) {
    let model = app.model.borrow();
    ui.summary
        .setStringValue(&NSString::from_str(&model.summary(&app.store)));
    ui.rename.setEnabled(model.can_edit());
    ui.delete.setEnabled(model.can_edit());
    show_tabs(app, ui);
    ui.preview.setNeedsDisplay(true);
}

fn redraw(_app: &App, ui: &DashboardUi) {
    ui.preview.setNeedsDisplay(true);
}

/// A click on a tab: the picture shows that desktop of that screen.
pub fn tab_chosen(app: &App, sender: &NSSegmentedControl) {
    let row = usize::try_from(sender.tag()).unwrap_or(usize::MAX);
    let tab = usize::try_from(sender.selectedSegment()).unwrap_or(usize::MAX);
    app.model.borrow_mut().choose(&app.store, row, tab);
    with_ui(app, redraw);
}

/// The window has a new size: the tabs are laid out for it.
pub fn resized(app: &App) {
    with_ui(app, show_tabs);
}

/// Makes the list match the model. (Selecting a row calls `selection_changed`, so the
/// model must not be borrowed while that happens.)
fn show_all(app: &App, ui: &DashboardUi) {
    let selected = app.model.borrow().selected_row();
    let rows = selected
        .map(NSIndexSet::indexSetWithIndex)
        .unwrap_or_else(NSIndexSet::indexSet);
    ui.table.reloadData();
    ui.table.selectRowIndexes_byExtendingSelection(&rows, false);
    show_details(app, ui);
}

pub fn open(app: &App) {
    let ui = app.dashboard.take().unwrap_or_else(|| build(app));
    app.model.replace(Dashboard::open(&app.store));
    #[allow(deprecated)] // `activate()` needs macOS 14; this also covers macOS 13.
    NSApplication::sharedApplication(app.mtm).activateIgnoringOtherApps(true);
    ui.window.makeKeyAndOrderFront(None);
    // Arrow keys move through the list straight away.
    ui.window.makeFirstResponder(Some(&ui.table));
    app.dashboard.replace(Some(ui));
    with_ui(app, show_all);
}

/// The window is closing: let go of its views. The window itself is released a moment
/// later, once AppKit has finished closing it.
pub fn closed(app: &App) {
    app.dashboard
        .take()
        .map(|ui| Retained::autorelease_ptr(ui.window));
}

pub fn row_count(app: &App) -> isize {
    app.model.borrow().rows() as isize
}

pub fn row_value(app: &App, row: isize) -> Option<Retained<AnyObject>> {
    let row = usize::try_from(row).ok()?;
    let title = NSString::from_str(&app.model.borrow().row_title(row));
    Some(Retained::into_super(Retained::into_super(title)))
}

fn follow_selection(app: &App, ui: &DashboardUi) {
    let row = usize::try_from(ui.table.selectedRow()).ok();
    app.model.borrow_mut().select(row);
    show_details(app, ui);
}

pub fn selection_changed(app: &App) {
    with_ui(app, follow_selection);
}

/// Runs a change that may show dialogs. The model is edited on a copy, so the window
/// can keep drawing the old state while a dialog is open.
fn change(app: &App, action: impl Fn(&mut Dashboard)) {
    let mut model = app.model.borrow().clone();
    action(&mut model);
    app.model.replace(model);
    with_ui(app, show_all);
}

pub fn rename(app: &App) {
    change(app, |model| model.rename_selected(&app.session()));
}

pub fn delete(app: &App) {
    change(app, |model| model.delete_selected(&app.session()));
}

/// Pixels per point in snapshots: sharp on any display, and the same on every machine.
const SNAPSHOT_SCALE: f64 = 2.0;

/// The view drawn into a PNG.
fn png_of(view: &NSView) -> Option<Retained<NSData>> {
    let bounds = view.bounds();
    // SAFETY: a null plane pointer asks AppKit to allocate the pixel buffer itself; the
    // remaining arguments describe ordinary 8-bit RGBA.
    let bitmap = unsafe {
        NSBitmapImageRep::initWithBitmapDataPlanes_pixelsWide_pixelsHigh_bitsPerSample_samplesPerPixel_hasAlpha_isPlanar_colorSpaceName_bytesPerRow_bitsPerPixel(
            NSBitmapImageRep::alloc(),
            std::ptr::null_mut(),
            (bounds.size.width * SNAPSHOT_SCALE) as isize,
            (bounds.size.height * SNAPSHOT_SCALE) as isize,
            8,
            4,
            true,
            false,
            NSDeviceRGBColorSpace,
            0,
            0,
        )
    }?;
    bitmap.setSize(bounds.size);
    view.cacheDisplayInRect_toBitmapImageRep(bounds, &bitmap);
    // SAFETY: an empty property dictionary is valid for PNG output.
    unsafe {
        bitmap.representationUsingType_properties(NSBitmapImageFileType::PNG, &NSDictionary::new())
    }
}

/// The whole window, title bar included (the content view's parent is the window frame).
fn frame_view(ui: &DashboardUi) -> Option<Retained<NSView>> {
    // SAFETY: reading the parent of a live view on the main thread.
    ui.window
        .contentView()
        .and_then(|content| unsafe { content.superview() })
}

fn write_png(_app: &App, ui: &DashboardUi, file: &str) {
    // Give the window a moment to become the active one, so it is drawn as such.
    NSRunLoop::currentRunLoop().runUntilDate(&NSDate::dateWithTimeIntervalSinceNow(0.6));
    ui.window.displayIfNeeded();
    frame_view(ui)
        .and_then(|frame| png_of(&frame))
        .map(|png| png.writeToFile_atomically(&NSString::from_str(file), true));
}

/// `--snapshot <file>`: draw the dashboard into a PNG file, then quit.
pub fn snapshot(app: &App, file: &str) {
    open(app);
    with_ui(app, |app, ui| write_png(app, ui, file));
    app.quit();
}
