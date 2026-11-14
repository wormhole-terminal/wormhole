// text-file: a test driver's replies on the control folder, not interface text
// The control folder's presses and read-outs of the right panel's parts that the earlier commands left out (`WORMHOLE_DEVCTL`),
// each through the closure or model the part itself calls:
//   `changes-rows`                 the Changes list as its view builds it: a card per section, folders, files (`ChangesRowsReadout`)
//   `panel-tabs`                   the panel's tab strip: each tab, its count badge and what VoiceOver says of it (the functions
//                                  `RightPanel` draws them from)
//   `submodule-enter <path>`       a submodule row's "Open a terminal in …" button (`SubmoduleRowView.enter`)
//   `diff-drawn`                   how the diff view laid the open diff out: split or unified, its rows, the switch, the column notes
//   `diff-layout split|unified`    the diff header's Split / Unified switch (there only where the view had room for it)
//   `review-paste` / `review-clear`  the review bar's Paste into Terminal and Clear (there only while the diff has comments)
//   `comment-remove`               the comment dialog's Remove (there only for a line that has a comment)
//   `task-move <from> <to>` / `note-move <from> <to>`  a next step or a note dropped on another (1-based), as `ReorderDrop` does:
//                                  the drop's result is the core's rule (`view::reorder::move_item`); the drag itself is a person's
//   `cursor [terminal]`            the cursor as that terminal's view draws it next (none: the selected one)

import AppKit
import DesignSystem
import PanelModel
import PanelUI
import WindowUI

extension AppController {
    /// False when `command` is not one of these.
    func runPanelActs(command: String, argument: String) -> Bool {
        let argument = argument.trimmingCharacters(in: .whitespaces)
        let panel = hub.panel, changes = hub.panel.changes
        switch command {
        case "changes-rows": reply(ChangesRowsReadout.text(changes))
        case "panel-tabs":
            reply(hub.panelTabs.map { tab in
                Readout.line(
                    "tab", [tab.rawValue], pairs: [("badge", panel.badge(for: tab).map(String.init) ?? "-"), ("spoken", hub.spokenTab(named: tab.rawValue))],
                    flags: [Readout.flag("shown", tab == hub.state.panelTab)])
            }.joined(separator: "\n"))
        case "submodule-enter":
            reply(panel.enterSubmodule(argument) ? "submodule-enter \(argument)" : "error: no submodule row \(argument) with a terminal button")
        case "diff-drawn": reply(DiffReadout.drawn(panel))
        case "diff-layout":
            // The header's switch, which it shows only with room for two columns and lines to lay out (`DiffHeader.splitFits`).
            let session = panel.tab == .history ? panel.history.diff : changes.diff
            guard ["split", "unified"].contains(argument) else { reply("error: diff-layout split|unified"); return true }
            guard session.drawn?.fits == true else { reply("error: the diff header has no Split / Unified switch"); return true }
            session.split = argument == "split"
            reply("diff-layout \(argument)")
        case "review-paste", "review-clear":
            // The bar is under the rows of a Changes diff against the working tree while the terminal has comments (`ChangesDiffPane`).
            guard changes.current != nil, changes.canReview, !changes.review.isEmpty else { reply("error: no review bar is shown"); return true }
            if command == "review-paste" { changes.pasteReview() } else { changes.review.clear() }
            reply("\(command) copied-instead=\(changes.copiedInstead)")
        case "comment-remove":
            // The dialog offers Remove only for a line that has a comment (`CommentEditor`).
            guard changes.commenting != nil, window?.attachedSheet != nil, changes.commentingExisting else { reply("error: no comment dialog with Remove is open"); return true }
            changes.removeComment()
            reply(changes.review.isEmpty ? "comment-remove: no comments" : "comment-remove \(changes.review.summary)")
        case "task-move", "note-move":
            let numbers = argument.split(separator: " ").compactMap { Int($0) }
            let notes = panel.notes
            let count = command == "task-move" ? notes.model.tasks.items.count : notes.model.notes.count
            guard numbers.count == 2, (1...max(count, 1)).contains(numbers[0]), (1...max(count, 1)).contains(numbers[1]), count > 0 else {
                reply("error: \(command) <from> <to> (1 to \(count))")
                return true
            }
            if command == "task-move" { notes.moveTask(from: numbers[0] - 1, to: numbers[1] - 1) } else { notes.moveNote(from: numbers[0] - 1, to: numbers[1] - 1) }
            reply("\(command) \(numbers[0]) \(numbers[1])")
        case "cursor":
            guard let id = Int(argument) ?? hub.state.selected, let view = hub.terminals.existing(id) else { reply("error: no terminal \(argument)"); return true }
            reply(view.cursorReadout)
        default: return false
        }
        return true
    }
}
