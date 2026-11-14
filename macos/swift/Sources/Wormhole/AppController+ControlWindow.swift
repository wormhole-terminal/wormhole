// The control folder's window commands (`WORMHOLE_DEVCTL`): everything a person does to the window that is not
// about one terminal's grid. Each does what the menu item, key or button does: it sends the same action.

import AppKit
import AuxModel
import Foundation
import PanesModel
import PlatformModel
import WindowUI
import WorkspaceModel

extension AppController {
    /// False when `command` is not one of the window's.
    func runWindow(command: String, argument: String) -> Bool {
        let hub = hub
        switch command {
        case "sidebar": hub.send(.toggleSidebar)
        case "panel": hub.send(.togglePanel)
        case "palette": router.run("commandPalette")
        case "shortcuts": router.run("keyboardShortcuts")
        case "settings": showSettings()
        case "new": hub.send(.newTerminal)
        case "close": closeTerminal(hub.state.selected ?? 0)
        case "select":
            guard let id = Int(argument) else { reply("error: select needs a terminal id"); return true }
            hub.send(.select(id))
        case "tab":
            guard let tab = PanelTab(rawValue: argument) else { reply("error: tab is notes, changes, history, usage, project or files"); return true }
            hub.send(.showPanelTab(tab))
        case "size":
            let sides = argument.split(separator: "x").compactMap { Double($0) }
            guard sides.count == 2 else { reply("error: size is WIDTHxHEIGHT"); return true }
            windowController?.setContentSize(asked: CGSize(width: sides[0], height: sides[1]))
        case "live-resize-sim": liveResizeSim(argument)
        case "split": hub.split(["down", "column"].contains(argument) ? .column : .row)
        case "edge":
            // `edge sidebar|panel <extent>`: a column's edge dragged to a width, as the handle does.
            let words = argument.split(separator: " ").map(String.init)
            guard words.count == 2, let extent = Double(words[1]), let edge = ["sidebar": ColumnEdge.sidebar, "panel": .panel][words[0]] else {
                reply("error: edge sidebar|panel <width>")
                return true
            }
            hub.send(.dragEdge(edge, extent: extent))
        case "zoom": hub.toggleZoom()
        case "join":
            guard let id = Int(argument), let anchor = hub.state.selected else { reply("error: join needs a terminal id"); return true }
            hub.send(.joinSplit(id, with: anchor))
        case "leave": if let id = hub.state.selected { hub.send(.leaveSplit(id)) }
        case "focus":
            guard let direction = Direction.allCases.first(where: { "\($0)" == argument }) else { reply("error: focus left, right, up or down"); return true }
            hub.focusPane(direction)
        case "action":
            guard !argument.isEmpty else { reply("error: action needs a keymap id"); return true }
            router.run(ActionID(rawValue: argument))
        case "window-shot":
            guard !argument.isEmpty else { reply("error: window-shot needs a path"); return true }
            let words = argument.split(separator: " ", maxSplits: 1).map(String.init)
            reply(windowShot(to: words[0], titled: words.count > 1 ? words[1] : nil))
            return true
        case "windows":
            reply(NSApp.windows.map { "\($0.title.isEmpty ? "(untitled)" : $0.title) visible=\($0.isVisible) sheet=\($0.attachedSheet != nil)" }.joined(separator: "\n"))
            return true
        case "menus":
            reply(menus?.dump() ?? "no menus")
            return true
        case "find": runFind(argument)
        case "diff": runDiff(argument)
        case "notice": reply(postNotice(argument)); return true
        // `onboarding <welcome|folder|look|agents|primer|close>`, as the GPUI app's devctl: the intro window at that step, or closed.
        case "onboarding" where !argument.isEmpty: aux.showOnboarding(step: argument)
        case "about", "licenses", "hosts", "trust", "access", "updates", "onboarding", "quick-panel": runAux(command)
        case "host":
            // What the bar's buttons do, for the selected terminal: retry, new, end, end-hard.
            guard let id = hub.state.selected else {
                reply("error: no terminal")
                return true
            }
            switch argument {
            case "retry": hub.store.reconnect(id)
            case "new": hub.restart(id)
            case "end": hub.endHost(id, hard: false)
            case "end-hard": hub.endHost(id, hard: true)
            default:
                reply("error: host retry|new|end|end-hard")
                return true
            }
        case "quit-ask", "quit-ask-end-all":
            // What ⌘Q (or Quit and End All Terminals) would put to the person now: the sheet's words, or that it quits at once.
            guard let question = quitFacts.question(endAll: command == "quit-ask-end-all") else { reply("quit-ask no question: nothing runs, the app quits at once"); return true }
            reply("quit-ask title=\(question.title)\ndetail=\(question.detail)\nconfirm=\(question.confirm) checkbox=\(question.checkbox.map { "\($0.label) checked=\($0.checked)" } ?? "none")")
            return true
        case "quit", "quit-keep", "quit-end-all":
            reply(command)
            terminateUnattended(endAll: command == "quit-end-all")
            return true
        default: return false
        }
        reply(argument.isEmpty ? command : "\(command) \(argument)")
        return true
    }

    /// `live-resize-sim <width> <height> <shot prefix>`: the window dragged from its size to this frame size in eight steps,
    /// each held to the grid as a real drag is (`windowWillResize`), with the drag's start and end notified. A shot of the
    /// window window-server composited is written mid-drag (`<prefix>-mid.png`) and when it ends (`<prefix>-end.png`).
    /// AppKit's own live resize cannot be started from outside, so `inLiveResize` stays false here.
    private func liveResizeSim(_ argument: String) {
        let words = argument.split(separator: " ").map(String.init)
        guard words.count == 3, let width = Double(words[0]), let height = Double(words[1]), let window, let controller = front.controller else {
            return reply("error: live-resize-sim WIDTH HEIGHT PREFIX")
        }
        let from = window.frame.size, to = CGSize(width: width, height: height)
        controller.windowWillStartLiveResize(Notification(name: NSWindow.willStartLiveResizeNotification))
        LiveResizeDriving.simulated = true
        Task { @MainActor in
            for step in 1...8 {
                let t = Double(step) / 8
                let proposed = CGSize(width: from.width + (to.width - from.width) * t, height: from.height + (to.height - from.height) * t)
                var frame = window.frame
                let size = controller.windowWillResize(window, to: proposed)
                frame.origin.y += frame.height - size.height
                frame.size = size
                window.setFrame(frame, display: true)
                window.contentView?.layoutSubtreeIfNeeded()
                reply("step \(step) \(selectedView.map(geometry) ?? "")")
                if step == 4 { reply(windowShot(to: words[2] + "-mid.png")) }
                reply("chip " + hub.sizeChips.joined(separator: ", "))
                try? await Task.sleep(for: .milliseconds(100))
            }
            try? await Task.sleep(for: .milliseconds(300))
            controller.windowDidEndLiveResize(Notification(name: NSWindow.didEndLiveResizeNotification))
            LiveResizeDriving.simulated = false
            try? await Task.sleep(for: .milliseconds(300))
            reply(selectedView.map(geometry) ?? "")
            reply(windowShot(to: words[2] + "-end.png"))
        }
    }

    private func runFind(_ text: String) {
        hub.find(.open)
        guard !text.isEmpty else { return }
        hub.setFindText(text)
    }

    /// `diff <n>` opens the n-th changed file over the terminals (1-based); `diff` alone closes it.
    private func runDiff(_ argument: String) {
        hub.send(.showPanelTab(.changes))
        hub.panel.tab = .changes
        let changes = hub.panel.changes
        guard let n = Int(argument), changes.files.indices.contains(n - 1) else { return changes.close() }
        changes.open(changes.files[n - 1].id)
    }

    private func runAux(_ command: String) {
        switch command {
        case "about": aux.showAbout()
        case "licenses": aux.showLicenses()
        case "hosts": aux.showHosts()
        case "trust": aux.showTrust()
        case "access": aux.showAccess()
        case "updates": aux.showUpdates()
        case "onboarding": aux.showOnboarding(firstRun: false)
        default: ambient?.toggleQuickPanel()
        }
    }

    /// A window as it is drawn (the workspace's, or the one with this title), for a person who cannot capture the screen: the content view cached into a PNG.
    func windowShot(to path: String, titled title: String? = nil) -> String {
        let shown = title.flatMap { wanted in NSApp.windows.first { $0.title == wanted } } ?? window
        guard let view = shown?.contentView, let bitmap = view.bitmapImageRepForCachingDisplay(in: view.bounds) else {
            return "error: no window"
        }
        view.cacheDisplay(in: view.bounds, to: bitmap)
        // `cacheDisplay` leaves out what the system draws in its own layers (a native sidebar, glass): the window server's
        // image of this process's own window has them, and needs no Screen Recording permission.
        let composited = shown.flatMap(Self.compositedImage)
        guard let png = composited?.representation(using: .png, properties: [:]) ?? bitmap.representation(using: .png, properties: [:]) else {
            return "error: no PNG"
        }
        do {
            try png.write(to: URL(fileURLWithPath: path))
            return "window-shot \(path)"
        } catch {
            return "error: window-shot failed: \(error)"
        }
    }

    /// The window as the window server composites it, by `CGWindowListCreateImage` looked up at run time (the SDK marks it unavailable).
    private static func compositedImage(of window: NSWindow) -> NSBitmapImageRep? {
        typealias Create = @convention(c) (CGRect, UInt32, UInt32, UInt32) -> Unmanaged<CGImage>?
        guard window.windowNumber > 0, let symbol = dlsym(UnsafeMutableRawPointer(bitPattern: -2), "CGWindowListCreateImage") else { return nil }
        let create = unsafeBitCast(symbol, to: Create.self)
        // optionIncludingWindow (8), bestResolution | boundsIgnoreFraming (1 | 4)... framing kept: the content is cropped below.
        guard let image = create(.null, 8, UInt32(window.windowNumber), 1 | 4)?.takeRetainedValue() else { return nil }
        return NSBitmapImageRep(cgImage: image)
    }
}
