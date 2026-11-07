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
            window?.setContentSize(CGSize(width: sides[0], height: sides[1]))
        case "split": hub.split(["down", "column"].contains(argument) ? .column : .row)
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
        case "notice": hub.notices.post(.copied(.characters(Int(argument) ?? 12)))
        case "about", "licenses", "hosts", "trust", "access", "updates", "onboarding", "quick-panel": runAux(command)
        case "quit":
            reply("quit")
            terminateUnattended()
            return true
        default: return false
        }
        reply(argument.isEmpty ? command : "\(command) \(argument)")
        return true
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
        guard let png = bitmap.representation(using: .png, properties: [:]) else { return "error: no PNG" }
        do {
            try png.write(to: URL(fileURLWithPath: path))
            return "window-shot \(path)"
        } catch {
            return "error: window-shot failed: \(error)"
        }
    }
}
