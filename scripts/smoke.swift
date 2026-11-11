// Drives the real menu bar app through the macOS Accessibility API and checks what
// happens: the menu's contents, saving, the dashboard, renaming, deleting, quitting.
//
//   swift scripts/smoke.swift <path to layoutsmart-app binary>
//
// Local use only: the terminal running it needs Accessibility permission, and dialogs
// will flash on screen for a few seconds. Profiles go to a temporary folder.

import ApplicationServices
import Foundation

setvbuf(stdout, nil, _IONBF, 0)

func fail(_ message: String) -> Never {
    print("FAIL: \(message)")
    exit(1)
}

func attribute(_ element: AXUIElement, _ name: String) -> CFTypeRef? {
    var value: CFTypeRef?
    return AXUIElementCopyAttributeValue(element, name as CFString, &value) == .success ? value : nil
}

func children(_ element: AXUIElement) -> [AXUIElement] {
    (attribute(element, "AXChildren") as? [AXUIElement]) ?? []
}

func role(_ element: AXUIElement) -> String { (attribute(element, "AXRole") as? String) ?? "" }
func title(_ element: AXUIElement) -> String { (attribute(element, "AXTitle") as? String) ?? "" }

/// Depth-first search for an element with this role (and title, if given).
func find(_ root: AXUIElement, role wanted: String, title wantedTitle: String? = nil, depth: Int = 8) -> AXUIElement? {
    if role(root) == wanted && (wantedTitle == nil || title(root) == wantedTitle) { return root }
    if depth == 0 { return nil }
    for child in children(root) {
        if let hit = find(child, role: wanted, title: wantedTitle, depth: depth - 1) { return hit }
    }
    return nil
}

func all(_ root: AXUIElement, role wanted: String, depth: Int = 8) -> [AXUIElement] {
    var hits = role(root) == wanted ? [root] : []
    if depth > 0 { for child in children(root) { hits += all(child, role: wanted, depth: depth - 1) } }
    return hits
}

/// What the last click reported (0 = delivered and answered), for failure reports.
var lastPress: Int32 = 0

func press(_ element: AXUIElement) {
    // The action may open a dialog and not answer until that closes, so the wait for an
    // answer is cut short. It cannot be much shorter: while a click is unanswered macOS
    // holds back every other request to the app, and on a slow machine a click that is
    // given up on too early may never be delivered at all.
    AXUIElementSetMessagingTimeout(element, 3)
    lastPress = AXUIElementPerformAction(element, "AXPress" as CFString).rawValue
}

/// Printed when something never shows up, so a failure on a remote machine can be understood.
var describeScreen: () -> String = { "" }

// Generous: shared CI machines can take many seconds to put up a first dialog.
func wait<T>(_ what: String, seconds: Double = 30, _ probe: () -> T?) -> T {
    let deadline = Date().addingTimeInterval(seconds)
    while Date() < deadline {
        if let value = probe() { return value }
        usleep(100_000)
    }
    fail("timed out waiting for \(what)\n\(describeScreen())")
}

func check(_ condition: Bool, _ what: String) {
    if !condition { fail(what) }
    print("  ok: \(what)")
}

guard CommandLine.arguments.count == 2 else { fail("usage: swift scripts/smoke.swift <layoutsmart-app binary>") }
guard AXIsProcessTrusted() else { fail("this terminal needs Accessibility permission") }

let folder = FileManager.default.temporaryDirectory.appendingPathComponent("layoutsmart-smoke-\(getpid())")
try? FileManager.default.createDirectory(at: folder, withIntermediateDirectories: true)
func profiles() -> [String] {
    ((try? FileManager.default.contentsOfDirectory(atPath: folder.path)) ?? [])
        .filter { $0.hasSuffix(".json") }.map { String($0.dropLast(5)) }.sorted()
}

let process = Process()
process.executableURL = URL(fileURLWithPath: CommandLine.arguments[1])
process.environment = ProcessInfo.processInfo.environment.merging(["LAYOUTSMART_DIR": folder.path]) { $1 }
try! process.run()
defer { if process.isRunning { process.terminate() } }
let app = AXUIElementCreateApplication(process.processIdentifier)

func statusItem() -> AXUIElement {
    wait("the menu bar icon") {
        (attribute(app, "AXExtrasMenuBar")).flatMap { children($0 as! AXUIElement).first }
    }
}

/// Opens the menu and returns its items.
func openMenu() -> [AXUIElement] {
    press(statusItem())
    let items: [AXUIElement] = wait("the menu to open") {
        let items = all(statusItem(), role: "AXMenuItem")
        return items.isEmpty ? nil : items
    }
    // Let the menu finish appearing; a click that arrives too early is ignored.
    usleep(500_000)
    return items
}

describeScreen = {
    let windows = (attribute(app, "AXWindows") as? [AXUIElement]) ?? []
    let lines = windows.map { window -> String in
        let buttons = all(window, role: "AXButton").map(title).filter { !$0.isEmpty }
        return "  window role=\(role(window)) subrole=\((attribute(window, "AXSubrole") as? String) ?? "") title='\(title(window))' buttons=\(buttons)"
    }
    let menu = all(statusItem(), role: "AXMenuItem").map(title)
    return "last click reported: \(lastPress)\napp running: \(process.isRunning)\nwindows (\(windows.count)):\n\(lines.joined(separator: "\n"))\nmenu items showing: \(menu)"
}

func dialog() -> AXUIElement {
    wait("a dialog") {
        ((attribute(app, "AXWindows") as? [AXUIElement]) ?? []).first { find($0, role: "AXButton", title: "Cancel") != nil || find($0, role: "AXButton", title: "OK") != nil }
    }
}

func answer(text: String?, button: String) {
    let window = dialog()
    if let text {
        let field = wait("the name box") { find(window, role: "AXTextField") }
        AXUIElementSetAttributeValue(field, "AXValue" as CFString, text as CFString)
    }
    press(wait("the \(button) button") { find(window, role: "AXButton", title: button) })
    usleep(700_000)
}

print("1. Menu with nothing saved")
var items = openMenu()
check(items.map(title) == ["Save Window Positions…", "", "No Saved Profiles", "", "Open Dashboard…", "", "Quit Layoutsmart"], "menu lists Save, an empty-state line, Dashboard and Quit")
check((attribute(items[2], "AXEnabled") as? Bool) == false, "'No Saved Profiles' is greyed out")

print("2. Save Window Positions… → Cancel")
press(items[0])
answer(text: nil, button: "Cancel")
check(profiles().isEmpty, "cancelling saves nothing")

print("3. Save with an invalid name, then a valid one")
press(openMenu()[0])
answer(text: "bad/name", button: "Save")
check(profiles().isEmpty, "a name with '/' is refused")
answer(text: "smoke test", button: "Save")
_ = wait("the profile file") { profiles() == ["smoke test"] ? true : nil }
check(true, "profile 'smoke test' was written")

print("4. The menu now lists the profile; clicking it restores without complaint")
items = openMenu()
check(items.map(title) == ["Save Window Positions…", "", "smoke test", "", "Open Dashboard…", "", "Quit Layoutsmart"], "menu lists the saved profile")
press(items[2])
usleep(700_000)
let stray = ((attribute(app, "AXWindows") as? [AXUIElement]) ?? []).filter { find($0, role: "AXButton", title: "OK") != nil }
check(stray.isEmpty, "restoring shows no error")

print("5. Saving under an existing name asks before replacing")
press(openMenu()[0])
answer(text: "smoke test", button: "Save")
answer(text: nil, button: "Cancel")          // "Replace?" → no; back to the name box
answer(text: nil, button: "Cancel")          // give up
check(profiles() == ["smoke test"], "declining to replace keeps the profile")

print("6. Dashboard")
press(openMenu()[4])
let dashboard: AXUIElement = wait("the dashboard window") {
    ((attribute(app, "AXWindows") as? [AXUIElement]) ?? []).first { title($0) == "Layoutsmart" }
}
let rows = wait("the profile list") { () -> [AXUIElement]? in
    let found = all(dashboard, role: "AXRow")
    return found.isEmpty ? nil : found
}
check(rows.count == 1, "the list shows one profile")
let summary = all(dashboard, role: "AXStaticText").compactMap { attribute($0, "AXValue") as? String }
check(summary.contains { $0.contains("display") && $0.contains("window") && $0.contains("saved") }, "the summary line describes the profile")

print("7. Rename")
press(wait("Rename…") { find(dashboard, role: "AXButton", title: "Rename…") })
answer(text: "renamed", button: "Rename")
_ = wait("the renamed file") { profiles() == ["renamed"] ? true : nil }
check(true, "the profile file was renamed")

print("8. Delete (asks first)")
press(wait("Delete…") { find(dashboard, role: "AXButton", title: "Delete…") })
answer(text: nil, button: "Cancel")
check(profiles() == ["renamed"], "declining keeps the profile")
press(wait("Delete…") { find(dashboard, role: "AXButton", title: "Delete…") })
answer(text: nil, button: "Delete")
_ = wait("the file to disappear") { profiles().isEmpty ? true : nil }
check(true, "confirming deletes the profile")
check((attribute(wait("Rename…") { find(dashboard, role: "AXButton", title: "Rename…") }, "AXEnabled") as? Bool) == false, "Rename is disabled when nothing is selected")

print("9. Quit")
items = openMenu()
press(items.last!)
_ = wait("the app to quit") { process.isRunning ? nil : true }
check(true, "Quit Layoutsmart ends the app")

try? FileManager.default.removeItem(at: folder)
print("PASS")
