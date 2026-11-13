// What the control folder reads of a window that the views keep to themselves (`WORMHOLE_DEVCTL`): the selected
// terminal's command blocks and pane bar, and the shown split's panes. Each prints what GPUI's devctl prints for the
// same command (`blocks`, `block <n> <action>`, `pane-bar`, `panes`); none has a rule of its own.

import Foundation
import TerminalUI
import WorkspaceModel

extension WindowHub {
    /// `blocks`: the selected terminal's blocks, one a line.
    public var blocksListing: String { selectedOverlay?.blocks.listing() ?? "" }

    /// `block <n> <select|copy-output|copy-command|to-prompt|fold>`: the Edit menu's action on the n-th block of `blocks`.
    /// Nil when it was done, else why not.
    public func blockAction(number: Int, named action: String) -> String? {
        guard let overlay = selectedOverlay else { return "no terminal" } // text: control state line
        let all = overlay.blocks.all()
        guard number >= 1, all.indices.contains(number - 1) else { return "no block \(number)" } // text: control state line
        let id = all[number - 1].id
        switch action {
        case "select": blockAction(.selectOutput, on: id)
        case "copy-output": blockAction(.copyOutput, on: id)
        case "copy-command": blockAction(.copyCommand, on: id)
        case "to-prompt": blockAction(.putAtPrompt, on: id)
        case "fold": foldOutput(block: id)
        default: return "expected select, copy-output, copy-command, to-prompt or fold, got \(action)" // text: control state line
        }
        return nil
    }

    /// `pane-bar`: what the bar over the selected terminal says (a program's question, a notice, a host's trouble), or `none`.
    public var paneBarSummary: String {
        guard let bars = selectedOverlay?.bars else { return "none" } // text: control state line
        var lines: [String] = []
        if let notice = bars.hostNotice { lines.append("banner \(notice)") } // text: control state line
        switch bars.bar {
        case .ask(let program)?: lines.append("question \(program) wants to read the clipboard") // text: control state line
        case .notice(let kind, let text)?: lines.append("notice \(kind) \(text)") // text: control state line
        case .hostUnavailable(let kind, let failure)?: lines.append("host-unavailable \(kind)\(failure.map { " \($0)" } ?? "")") // text: control state line
        case .hostNotResponding(let failure)?: lines.append("host-not-responding\(failure.map { " \($0)" } ?? "")") // text: control state line
        case nil: break
        }
        if bars.exited { lines.append("exited") } // text: control state line
        return lines.isEmpty ? "none" : lines.joined(separator: "\n") // text: control state line
    }

    /// `panes`: the shown split's panes and dividers with their rectangles, as GPUI prints them.
    public var panesListing: String {
        let panes = state.panes
        let preset = state.layout?.menu.presets.first { $0.current }?.id.rawValue ?? "none"
        var lines = ["split=\(panes.isSplit) layout=\(preset)"] // text: control state line
        for place in panes.shown {
            let r = place.rect
            lines.append("pane \(place.id) \(r.x) \(r.y) \(r.w) \(r.h)\(panes.focused == place.id ? " focused" : "")\(panes.zoomed == place.id ? " zoomed" : "")") // text: control state line
        }
        for divider in panes.dividers {
            let r = divider.rect
            lines.append("divider \(divider.path)/\(divider.index) \(divider.axis) at \(divider.position) \(r.x) \(r.y) \(r.w) \(r.h)") // text: control state line
        }
        return lines.joined(separator: "\n")
    }
}
