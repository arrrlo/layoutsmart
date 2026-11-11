// A throwaway app with one ordinary window, for trying Layoutsmart on something that is
// safe to move, resize and send into full screen.
//
//   swiftc -O scripts/testwindow.swift -o LayoutsmartTestWindow && ./LayoutsmartTestWindow
//
// It shows up in `layoutsmart list` as "LayoutsmartTestWindow". Quit it with Control-C.
import AppKit

let app = NSApplication.shared
app.setActivationPolicy(.regular)
let window = NSWindow(
    contentRect: NSRect(x: 160, y: 200, width: 560, height: 360),
    styleMask: [.titled, .closable, .resizable, .miniaturizable],
    backing: .buffered,
    defer: false
)
window.title = "Layoutsmart test window"
window.isReleasedWhenClosed = false
window.collectionBehavior = [.fullScreenPrimary]
window.orderFront(nil)
app.run()
