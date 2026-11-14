// text-file: a test driver's replies on the control folder, not interface text
// The control folder's reads and presses of the window's own small parts (`WORMHOLE_DEVCTL`), each through the closure or
// model the part itself calls:
//   `find-state`                     the find bar: `find-state open=<b> query="<q>" count="<words>" case=<b> word=<b> regex=<b>`
//   `branch-chip [<terminal>|state]` the title row's branch chip clicked for that terminal (none: the selected one), as
//                                    `TitleActions.pickBranch` does; `state` says `branch-chip picker=<terminal|->`
//   `group-popover <group>`          the group's popover in the list, as "Other colour…" opens it (`popover` reads it)
//   `sidebar-tip [close|never|keep|settings]`  the list's "No coding agents yet" note: `sidebar-tip <off|note|offer>`, and its
//                                    buttons (`SideColumns`: × closes it to the offer, Don't show again, keep, Coding agents settings)
//   `plus-menu [pick <key>]`         the + menu "New terminal with": `plus-menu` then `<key> "<name>" [default]` a line; a choice
//   `comment-at <row>`               the comment dialog of the diff's row `row` (as `diff-spans` numbers it) opened, as a click
//                                    on the line opens it; `comment <text>` then saves it
//   `review`                         what pasting the review would paste (`Review.message`), or `review: none`

import AppKit
import DesignSystem
import PanelModel
import SidebarUI
import TerminalUI
import WindowUI
import WorkspaceModel

extension AppController {
    /// False when `command` is not one of these.
    func runChromeActs(command: String, argument: String) -> Bool {
        let argument = argument.trimmingCharacters(in: .whitespaces)
        switch command {
        case "find-state":
            guard let find = hub.selectedFind else { reply("error: no terminal is shown"); return true }
            reply(Readout.line(
                "find-state", pairs: [("query", find.text), ("count", find.countText)],
                flags: [find.isOpen ? "open" : "closed", Readout.flag("case", find.options.matchCase), Readout.flag("word", find.options.wholeWord), Readout.flag("regex", find.options.regex)]))
        case "branch-chip":
            if argument == "state" { reply("branch-chip picker=\(hub.panel.changes.pickerFor.map(String.init) ?? "-")"); return true }
            guard let id = Int(argument) ?? hub.state.selected else { reply("error: no terminal"); return true }
            hub.actions.pickBranch(id)
            reply("branch-chip \(id)")
        case "group-popover":
            guard let id = GroupID(argument) else { reply("error: group-popover <group>"); return true }
            hub.sidebarLink.ask(.popover(id))
            reply("group-popover \(id)")
        case "sidebar-tip": reply(sidebarTip(argument))
        case "plus-menu":
            let shells = hub.actions.shells()
            let words = argument.split(separator: " ", maxSplits: 1).map(String.init)
            if words.first == "pick", words.count == 2 {
                guard shells.contains(where: { $0.key == words[1] }) else { reply("error: no shell \(words[1])"); return true }
                hub.actions.newTerminalWith(words[1])
                reply("plus-menu picked \(words[1])")
                return true
            }
            reply((["plus-menu"] + shells.map { Readout.line($0.key.isEmpty ? "-" : $0.key, quoted: [$0.name], flags: [Readout.flag("default", $0.isDefault)]) }).joined(separator: "\n"))
        case "comment-at": reply(commentAt(argument))
        case "review": reply(hub.panel.changes.review.message.map { "review\n" + $0 } ?? "review: none")
        default: return false
        }
        return true
    }

    /// The list's note after a first run without agents, as `SideColumns` shows it and wires its buttons.
    private func sidebarTip(_ argument: String) -> String {
        // An agent on screen takes the note away (`SideColumns`: `stage`).
        let stage = hub.state.all.contains { $0.agent != nil } ? AgentsTip.off : hub.agentsTip
        switch argument {
        case "": return "sidebar-tip \(stage)"
        case "close": hub.agentsTip = .offer
        case "never":
            hub.actions.dismissAgentsTip()
            hub.agentsTip = .off
        case "keep": hub.agentsTip = .off
        case "settings": hub.actions.openSettingsAt("agents")
        default: return "error: sidebar-tip [close|never|keep|settings]"
        }
        return "sidebar-tip \(argument)"
    }

    /// The comment dialog of a row of the open Changes diff, with the comment already there if the line has one.
    private func commentAt(_ argument: String) -> String {
        let changes = hub.panel.changes
        guard let row = Int(argument), let file = changes.current, let rows = changes.diff.built?.unified, rows.indices.contains(row),
            case .unified(let cell) = rows[row]
        else { return "error: comment-at <row of diff-spans that is a line>" }
        let side: LineSide = cell.kind == .removed ? .old : .new
        guard let line = side == .old ? cell.old : cell.new else { return "error: row \(row) has no line number" }
        let existing = changes.review.on(file.path, side, line)
        changes.commenting = ReviewComment(path: file.path, side: side, line: line, code: cell.text, text: existing?.text ?? "")
        return "comment-at \(file.path):\(line)"
    }
}
