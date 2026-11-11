// A throwaway app with one ordinary window, for trying Layoutsmart on something that is
// safe to move, resize and send into full screen.
//
//   swiftc -O scripts/testwindow.swift -o LayoutsmartTestWindow && ./LayoutsmartTestWindow
//
// It shows up in `layoutsmart list` as "LayoutsmartTestWindow". Quit it with Control-C.
// The window can also be told to go into full screen by itself, the way its green button
// does, to find out whether a Mac can do that at all:
//
//   kill -USR1 <pid>    enter full screen
//   kill -USR2 <pid>    leave full screen
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

/// Makes the window full screen, or not, unless it already is.
func setFullScreen(_ wanted: Bool) {
    if window.styleMask.contains(.fullScreen) != wanted {
        window.toggleFullScreen(nil)
    }
}

var sources: [DispatchSourceSignal] = []
for (number, wanted) in [(SIGUSR1, true), (SIGUSR2, false)] {
    signal(number, SIG_IGN)
    let source = DispatchSource.makeSignalSource(signal: number, queue: .main)
    source.setEventHandler { setFullScreen(wanted) }
    source.resume()
    sources.append(source)
}
app.run()
