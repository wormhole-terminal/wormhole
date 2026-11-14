// text-file: replies of the control folder and the idle check (dev tooling, English by design), not interface text
// The workspace from the control folder, for a driver that has no pointer: the same actions a click or a key sends, and a
// text dump of what the sidebar was given (the core's rows, words and numbers), so a script can check them.

import AppKit
import Foundation
import WorkspaceModel

extension AppController {
    /// Whether `command` was one of these.
    func runWorkspace(command: String, argument: String) -> Bool {
        let numbers = argument.split(separator: " ").compactMap { Int($0) }
        let parts = argument.split(separator: " ", maxSplits: 1)
        /// An action over the first number given, or the usage when there is none.
        func with(_ usage: String, _ make: (Int) -> WorkspaceAction) {
            if let first = numbers.first { send(make(first)) } else { reply("error: usage \(usage)") }
        }
        switch command {
        case "rows": reply(rows())
        case "rename":
            guard let id = parts.first.flatMap({ Int($0) }), parts.count == 2 else { reply("error: usage rename <id> <name>"); break }
            send(.rename(id, String(parts[1])))
        case "pin": with("pin <id>") { .togglePin($0) }
        case "group": send(.newGroup(numbers))
        case "fold": with("fold <group>") { .toggleGroup(GroupID($0)) }
        case "ungroup": with("ungroup <group>") { .ungroup(GroupID($0)) }
        case "groupname":
            guard let id = parts.first.flatMap({ GroupID($0) }), parts.count == 2 else { reply("error: usage groupname <group> <name>"); break }
            send(.renameGroup(id, String(parts[1])))
        case "ingroup":
            guard numbers.count == 2 else { reply("error: usage ingroup <id> <group>"); break }
            send(.addToGroup(numbers[0], GroupID(numbers[1])))
        // As the sidebar's + in the group's header: the group's folder is asked about first when it has none to offer.
        case "newin":
            if let first = numbers.first { hub.newTerminal(inGroup: GroupID(first)) } else { reply("error: usage newin <group>") }
        case "groupwindow": with("groupwindow <group>") { .moveGroupToWindow(GroupID($0)) }
        case "step": with("step <delta>") { .selectStep($0) }
        case "number": with("number <n>") { .selectNumber($0) }
        case "search": send(.search(argument))
        case "drop", "plan": drop(command == "plan", argument)
        default: return false
        }
        return true
    }

    private func send(_ action: WorkspaceAction) {
        let refusal = hub.store.send(action)
        reply("\(action) \(refusal.map { "refused \($0.rawValue)" } ?? "ok")")
    }

    /// `row:<id>:<part>`, `head:<group>:<part>`, `after-loose|after-split:<id>`, `after-group:<group>`, `tail`.
    private func over(_ text: Substring) -> DropOver? {
        let parts = text.split(separator: ":").map(String.init)
        func part(_ name: String?) -> DropPart? { ["top": .top, "middle": .middle, "bottom": .bottom][name ?? ""] }
        switch parts.first {
        case "row": if parts.count == 3, let id = Int(parts[1]), let part = part(parts[2]) { return .row(id, part) }
        case "head": if parts.count == 3, let id = GroupID(parts[1]), let part = part(parts[2]) { return .head(id, part) }
        case "after-loose": if parts.count == 2, let id = Int(parts[1]) { return .after(.loose(id)) }
        case "after-split": if parts.count == 2, let id = Int(parts[1]) { return .after(.split(id)) }
        case "after-group": if parts.count == 2, let id = GroupID(parts[1]) { return .after(.group(id)) }
        case "tail": return .tail
        default: break
        }
        return nil
    }

    /// `drop <id> <over>` lets a terminal go; `plan <id> <over>` only asks what that would do.
    private func drop(_ planOnly: Bool, _ argument: String) {
        let parts = argument.split(separator: " ")
        guard parts.count == 2, let id = Int(parts[0]), let target = over(parts[1]) else {
            return reply("error: usage drop|plan <id> row:<id>:<top|middle|bottom> | head:<g>:<part> | after-loose:<id> | tail")
        }
        if planOnly { return reply("plan \(hub.store.dropPlan(dragged: id, over: target).map { "\($0)" } ?? "nothing") ") }
        send(.dropTerminal(id, over: target))
    }

    /// The sidebar as the views were given it: one line per head and per row.
    private func rows() -> String {
        let state = hub.store.state
        var lines = ["selected=\(state.selected.map(String.init) ?? "none") live=\(state.live) groups=\(state.groups.count)"]
        for section in state.sections {
            if let head = section.head {
                lines.append("group \(head.id) \"\(head.name)\" folded=\(head.isFolded) pinned=\(head.isPinned) count=\(section.terminalCount) alerts=\(section.alertCount) spoken=\"\(head.spoken)\"")
            }
            for row in section.rows {
                let marks = row.splitMarks
                lines.append(
                    "  row \(row.id) \"\(row.terminal.title)\" \(row.terminal.activity) number=\(row.number.map(String.init) ?? "-") "
                        + "pinned=\(row.terminal.isPinned) folder=\(row.terminal.folder ?? "-") line=\"\(row.secondLine.text)\" "
                        + "trail=\(row.secondLine.trail ?? "-") split=\(marks.members)\(marks.bracket.map { ":\($0)" } ?? "") spoken=\"\(row.spoken)\"")
            }
        }
        return lines.joined(separator: "\n")
    }
}
