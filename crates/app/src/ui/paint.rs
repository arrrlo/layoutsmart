//! Draws what `layoutsmart_core::dashboard` computed: screens, windows and captions.

use super::state::App;
use layoutsmart_core::dashboard::{PALETTE_SIZE, Shape, ShapeKind};
use layoutsmart_core::geometry::{Rect, Size};
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2_app_kit::{
    NSBezierPath, NSColor, NSFont, NSFontAttributeName, NSForegroundColorAttributeName,
    NSGraphicsContext, NSLineBreakMode, NSMutableParagraphStyle, NSParagraphStyleAttributeName,
    NSRectClip, NSRectFill, NSStringDrawing, NSTextAlignment,
};
use objc2_foundation::{NSDictionary, NSPoint, NSRect, NSSize, NSString};

/// One colour per app, reused in order when there are more apps than colours.
const PALETTE: [(f64, f64, f64); PALETTE_SIZE] = [
    (0.26, 0.52, 0.96),
    (0.20, 0.66, 0.33),
    (0.98, 0.55, 0.16),
    (0.61, 0.35, 0.85),
    (0.10, 0.65, 0.70),
    (0.91, 0.30, 0.55),
    (0.80, 0.62, 0.08),
    (0.86, 0.26, 0.21),
];

fn srgb(red: f64, green: f64, blue: f64, alpha: f64) -> Retained<NSColor> {
    NSColor::colorWithSRGBRed_green_blue_alpha(red, green, blue, alpha)
}

fn screen_fill(_color: usize) -> Retained<NSColor> {
    srgb(0.09, 0.10, 0.12, 1.0)
}

fn window_fill(color: usize) -> Retained<NSColor> {
    let (red, green, blue) = PALETTE[color % PALETTE_SIZE];
    srgb(red, green, blue, 0.86)
}

fn no_label(_shape: &Shape, _rect: NSRect) {}

fn window_label(shape: &Shape, rect: NSRect) {
    let white = srgb(1.0, 1.0, 1.0, 1.0);
    let faded = srgb(1.0, 1.0, 1.0, 0.75);
    let name = NSRect::new(
        NSPoint::new(rect.origin.x + 6.0, rect.origin.y + 4.0),
        NSSize::new(rect.size.width - 12.0, 15.0),
    );
    let title = NSRect::new(
        NSPoint::new(name.origin.x, name.origin.y + 15.0),
        NSSize::new(name.size.width, 14.0),
    );
    clipped(rect, || {
        text(
            &shape.label,
            name,
            &NSFont::boldSystemFontOfSize(11.0),
            &white,
            NSTextAlignment::Left,
        );
        text(
            &shape.detail,
            title,
            &NSFont::systemFontOfSize(10.0),
            &faded,
            NSTextAlignment::Left,
        );
    });
}

/// How each kind of shape is drawn, indexed by `ShapeKind`.
struct Style {
    fill: fn(usize) -> Retained<NSColor>,
    stroke: (f64, f64, f64, f64),
    radius: f64,
    label: fn(&Shape, NSRect),
}

const STYLES: [Style; 2] = [
    Style {
        fill: screen_fill,
        stroke: (0.45, 0.47, 0.52, 1.0),
        radius: 6.0,
        label: no_label,
    },
    Style {
        fill: window_fill,
        stroke: (1.0, 1.0, 1.0, 0.55),
        radius: 3.0,
        label: window_label,
    },
];

fn ns_rect(rect: Rect) -> NSRect {
    NSRect::new(
        NSPoint::new(rect.x, rect.y),
        NSSize::new(rect.width, rect.height),
    )
}

/// Runs `draw` with drawing limited to `rect`.
fn clipped(rect: NSRect, draw: impl Fn()) {
    let context = NSGraphicsContext::currentContext();
    context
        .iter()
        .for_each(|context| context.saveGraphicsState());
    NSRectClip(rect);
    draw();
    context
        .iter()
        .for_each(|context| context.restoreGraphicsState());
}

fn text(string: &str, rect: NSRect, font: &NSFont, color: &NSColor, alignment: NSTextAlignment) {
    let style = attributes(font, color, alignment);
    // SAFETY: drawing a string with a valid attribute dictionary.
    unsafe { NSString::from_str(string).drawInRect_withAttributes(rect, Some(&style)) };
}

fn shape(shape: &Shape) {
    let style = &STYLES[shape.kind as usize];
    let rect = ns_rect(shape.rect);
    let path =
        NSBezierPath::bezierPathWithRoundedRect_xRadius_yRadius(rect, style.radius, style.radius);
    (style.fill)(shape.color).setFill();
    path.fill();
    let (red, green, blue, alpha) = style.stroke;
    srgb(red, green, blue, alpha).setStroke();
    path.setLineWidth(1.0);
    path.stroke();
    (style.label)(shape, rect);
}

fn attributes(
    font: &NSFont,
    color: &NSColor,
    alignment: NSTextAlignment,
) -> Retained<NSDictionary<NSString, AnyObject>> {
    let paragraph = NSMutableParagraphStyle::new();
    paragraph.setLineBreakMode(NSLineBreakMode::ByTruncatingTail);
    paragraph.setAlignment(alignment);
    let values: [&AnyObject; 3] = [font.as_ref(), color.as_ref(), paragraph.as_ref()];
    // SAFETY: the keys are AppKit's attribute-name constants, paired with the value types
    // AppKit documents for them (font, colour, paragraph style).
    unsafe {
        let keys = [
            NSFontAttributeName,
            NSForegroundColorAttributeName,
            NSParagraphStyleAttributeName,
        ];
        NSDictionary::from_slices(&keys, &values)
    }
}

/// A screen's name and size on a small badge that sits on its bottom border, so it stays
/// readable whatever windows are behind it.
fn caption(shape: &Shape) {
    let screen = ns_rect(shape.rect);
    let words = NSString::from_str(&format!("{}  ·  {}", shape.label, shape.detail));
    let style = attributes(
        &NSFont::systemFontOfSize(10.0),
        &srgb(1.0, 1.0, 1.0, 0.9),
        NSTextAlignment::Center,
    );
    // SAFETY: measuring and drawing a string with a valid attribute dictionary.
    let wanted = unsafe { words.sizeWithAttributes(Some(&style)) }.width + 18.0;
    let width = wanted.min(screen.size.width - 8.0);
    let badge = NSRect::new(
        NSPoint::new(
            screen.origin.x + (screen.size.width - width) / 2.0,
            screen.origin.y + screen.size.height - 9.0,
        ),
        NSSize::new(width, 18.0),
    );
    let pill = NSBezierPath::bezierPathWithRoundedRect_xRadius_yRadius(badge, 9.0, 9.0);
    srgb(0.20, 0.21, 0.24, 1.0).setFill();
    pill.fill();
    srgb(0.45, 0.47, 0.52, 1.0).setStroke();
    pill.stroke();
    let line = NSRect::new(
        NSPoint::new(badge.origin.x + 6.0, badge.origin.y + 2.5),
        NSSize::new(width - 12.0, 14.0),
    );
    // SAFETY: as above.
    unsafe { words.drawInRect_withAttributes(line, Some(&style)) };
}

/// Shown in the middle when there is no picture (nothing saved, nothing selected…).
fn message(words: &str, bounds: NSRect) {
    let area = NSRect::new(
        NSPoint::new(40.0, bounds.size.height / 2.0 - 24.0),
        NSSize::new(bounds.size.width - 80.0, 60.0),
    );
    text(
        words,
        area,
        &NSFont::systemFontOfSize(13.0),
        &NSColor::secondaryLabelColor(),
        NSTextAlignment::Center,
    );
}

pub fn preview(app: &App, bounds: NSRect) {
    let canvas = Size::new(bounds.size.width, bounds.size.height);
    let scene = app.model.borrow().scene(&app.store, canvas);
    NSColor::underPageBackgroundColor().setFill();
    NSRectFill(bounds);
    scene.shapes.iter().for_each(shape);
    scene
        .shapes
        .iter()
        .filter(|shape| shape.kind == ShapeKind::Display)
        .for_each(caption);
    message(&scene.message, bounds);
}
