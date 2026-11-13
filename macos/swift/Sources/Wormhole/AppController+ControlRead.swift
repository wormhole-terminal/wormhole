// The control folder's commands that read what the window holds and the few that drive it by row (`WORMHOLE_DEVCTL`), named
// and printed as GPUI's devctl has them (`devctl/mod.rs`): `blocks`, `block <n> <action>`, `images`, `pane-bar`, `panes`,
// `layout`, `shells`, `zen`, `new [dir]`, `new-shell`, `close <n>`, `agent-hide`. Each calls what the menu, a click or a key
// calls (`WindowHub`, `setZen`); none decides anything. Rows are 1-based, in the order the sidebar lists them (`state.all`).

import AppKit
import Foundation
import TerminalMetal
import WindowUI
import WorkspaceModel

extension AppController {
    /// False when `command` is not one of these.
    func runRead(command: String, argument: String) -> Bool {
        let argument = argument.trimmingCharacters(in: .whitespaces)
        switch command {
        case "blocks": reply(hub.blocksListing)
        case "block" where !argument.isEmpty:
            // The no-argument `block` says which block the Edit menu would act on (`blockTargetSummary`).
            let words = argument.split(separator: " ").map(String.init)
            guard words.count == 2, let n = Int(words[0]) else { reply("error: expected <n> <select|copy-output|copy-command|to-prompt|fold>"); return true }
            reply(hub.blockAction(number: n, named: words[1]).map { "error: \($0)" } ?? "block \(argument)")
        case "images": reply(imagesListing())
        case "pane-bar": reply(hub.paneBarSummary)
        case "panes": reply(hub.panesListing)
        case "layout":
            guard let preset = ["columns": LayoutPresetID.twoColumns, "rows": .twoRows, "grid": .grid, "single": .single][argument] else {
                reply("error: expected columns|rows|grid|single, got \(argument)")
                return true
            }
            hub.send(.applyLayout(preset))
            reply("layout \(argument)")
        case "shells":
            // key, name, what it runs, `default` on the default: the keys `new-shell` takes (`-` is the login shell).
            reply(settingsModel.host.shellChoices().map { "\($0.key.isEmpty ? "-" : $0.key)\t\($0.name)\t\($0.commandLine)\($0.isDefault ? "\tdefault" : "")" }.joined(separator: "\n"))
        case "zen":
            switch argument {
            case "": break
            case "on": setZen(true)
            case "off": setZen(false)
            case "toggle": toggleZen()
            default: reply("error: expected on|off|toggle, got \(argument)"); return true
            }
            reply("zen \(zenOn ? "on" : "off")")
        case "new" where !argument.isEmpty:
            // `new <dir>`: a terminal in that folder (`~/` is the home folder), as a worktree's new terminal opens.
            hub.send(.newTerminalIn(NSString(string: argument).expandingTildeInPath))
            reply("new \(argument)")
        case "new-shell":
            // `new-shell <key> [dir]`: a terminal running the shell the key names (`shells` lists the keys), in `dir` (`~/` is
            // the home folder) or where the selected one is.
            let parts = argument.split(separator: " ", maxSplits: 1).map { $0.trimmingCharacters(in: .whitespaces) }
            guard let first = parts.first, !first.isEmpty else {
                reply("error: expected <key> [dir] (`shells` lists the keys; `-` is the login shell)")
                return true
            }
            let key = first == "-" ? "" : first
            if parts.count == 2, !parts[1].isEmpty {
                hub.send(.newTerminalWithShellIn(key: key, folder: NSString(string: parts[1]).expandingTildeInPath))
            } else {
                hub.send(.newTerminalWithShell(key))
            }
            reply("new-shell \(argument)")
        case "close" where Int(argument) != nil:
            // `close <n>`: row n closes at once, without the question the × asks (a no-argument `close` is the selected one's).
            guard let id = row(argument) else { reply("error: no row \(argument)"); return true }
            finishClosing(id, in: session(holding: id) ?? front)
            reply("close \(argument)")
        case "agent-hide":
            let words = argument.split(separator: " ").map(String.init)
            guard let id = words.first.flatMap(row), words.count <= 2, ["on", "off"].contains(words.last ?? "on") || words.count == 1 else {
                reply("error: agent-hide <n> [on|off]")
                return true
            }
            let now = hub.state.all.first { $0.id == id }?.isAgentHidden ?? false
            hub.send(.setAgentHidden(id, words.count == 2 ? words[1] == "on" : !now))
            reply("agent-hide \(argument)")
        default: return false
        }
        return true
    }

    /// The terminal in row `n` (1-based) as the sidebar lists them, nil when there is none.
    private func row(_ n: String) -> TerminalID? {
        guard let n = Int(n) else { return nil }
        let all = hub.state.all
        return all.indices.contains(n - 1) ? all[n - 1].id : nil
    }

    /// `images`: the pictures the last frame placed, GPUI's shape: a total, then one line per picture (key, size, decoded bytes
    /// or `decoding`) and one per placement under it. Counted from the frame's placements: the engine's store is not asked.
    private func imagesListing() -> String {
        guard let view = selectedView else { return "error: no terminal is shown" }
        let placements = view.renderer.imagePlacements
        let keys = Set(placements.map(\.key)).sorted()
        var lines: [String] = []
        var total = 0
        for key in keys {
            guard let image = view.source.inlineImage(key: key) else { lines.append("\(key): decoding"); continue }
            total += image.rgba.count
            lines.append("\(key): \(image.width)x\(image.height) \(image.rgba.count) bytes")
            for place in placements where place.key == key {
                lines.append("  placement line \(place.line) column \(place.column) cells \(place.cols)x\(place.rows) z \(place.z)")
            }
        }
        return (["\(keys.count) images, \(total) bytes"] + lines).joined(separator: "\n")
    }
}
