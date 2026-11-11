// Draws the Layoutsmart app icon and writes packaging/AppIcon.icns.
//
//   swift packaging/make-icon.swift
//
// Only needed when the icon design changes; the generated .icns is committed.

import AppKit

let canvas = 1024
let bitmap = NSBitmapImageRep(
    bitmapDataPlanes: nil, pixelsWide: canvas, pixelsHigh: canvas, bitsPerSample: 8,
    samplesPerPixel: 4, hasAlpha: true, isPlanar: false, colorSpaceName: .deviceRGB,
    bytesPerRow: 0, bitsPerPixel: 0)!
NSGraphicsContext.current = NSGraphicsContext(bitmapImageRep: bitmap)

func withShadow(blur: CGFloat, drop: CGFloat, alpha: CGFloat, _ draw: () -> Void) {
    NSGraphicsContext.saveGraphicsState()
    let shadow = NSShadow()
    shadow.shadowColor = NSColor(white: 0, alpha: alpha)
    shadow.shadowBlurRadius = blur
    shadow.shadowOffset = NSSize(width: 0, height: -drop)
    shadow.set()
    draw()
    NSGraphicsContext.restoreGraphicsState()
}

// The rounded square every macOS icon sits on (824 pt inside a 1024 pt canvas).
let body = NSBezierPath(roundedRect: NSRect(x: 100, y: 100, width: 824, height: 824), xRadius: 185, yRadius: 185)
withShadow(blur: 26, drop: 14, alpha: 0.35) {
    NSColor(srgbRed: 0.24, green: 0.32, blue: 0.88, alpha: 1).setFill()
    body.fill()
}
NSGradient(
    starting: NSColor(srgbRed: 0.36, green: 0.62, blue: 1.00, alpha: 1),
    ending: NSColor(srgbRed: 0.24, green: 0.25, blue: 0.84, alpha: 1))!
    .draw(in: body, angle: -90)

// Three windows, each in its place: one tall on the left, two stacked on the right.
func window(_ rect: NSRect, alpha: CGFloat) {
    let path = NSBezierPath(roundedRect: rect, xRadius: 38, yRadius: 38)
    withShadow(blur: 18, drop: 8, alpha: 0.22) {
        NSColor(white: 1, alpha: alpha).setFill()
        path.fill()
    }
}
window(NSRect(x: 236, y: 236, width: 300, height: 552), alpha: 0.97)
window(NSRect(x: 566, y: 528, width: 222, height: 260), alpha: 0.80)
window(NSRect(x: 566, y: 236, width: 222, height: 262), alpha: 0.62)

NSGraphicsContext.current?.flushGraphics()

// Build the .iconset in a temporary folder, then pack it.
let folder = FileManager.default.temporaryDirectory.appendingPathComponent("Layoutsmart-\(getpid()).iconset")
try! FileManager.default.createDirectory(at: folder, withIntermediateDirectories: true)
let master = folder.appendingPathComponent("master.png")
try! bitmap.representation(using: .png, properties: [:])!.write(to: master)

func run(_ tool: String, _ arguments: [String]) {
    let process = Process()
    process.executableURL = URL(fileURLWithPath: tool)
    process.arguments = arguments
    process.standardOutput = FileHandle.nullDevice
    try! process.run()
    process.waitUntilExit()
    precondition(process.terminationStatus == 0, "\(tool) failed")
}

for points in [16, 32, 128, 256, 512] {
    for scale in [1, 2] {
        let pixels = points * scale
        let name = scale == 1 ? "icon_\(points)x\(points).png" : "icon_\(points)x\(points)@2x.png"
        run("/usr/bin/sips", ["-z", "\(pixels)", "\(pixels)", master.path, "--out", folder.appendingPathComponent(name).path])
    }
}
try! FileManager.default.removeItem(at: master)
run("/usr/bin/iconutil", ["-c", "icns", folder.path, "-o", "packaging/AppIcon.icns"])
try! FileManager.default.removeItem(at: folder)
print("wrote packaging/AppIcon.icns")
