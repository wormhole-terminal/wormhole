// text-file: a test driver's replies on the control folder, not interface text
// The control folder's reads and presses of the window's own small parts (`WORMHOLE_DEVCTL`), each through the closure or
// model the part itself calls:
//   `find-state`                     the find bar: `find-state open=<b> query="<q>" count="<words>" case=<b> word=<b> regex=<b>`
//   `branch-chip [<terminal>|state]` the title row's branch chip clicked for that terminal (none: the selected one), as
//                                    `TitleActions.pickBranch` does; `state` says `branch-chip picker=<terminal|-> list=<open|closed>`
//   `group-popover <group>`          the group's popover in the list, as "Other colour…" opens it (`popover` reads it)
//   `sidebar-tip [close|never|keep|settings]`  the list's "No coding agents yet" note: `sidebar-tip <off|note|offer>`, and its
//                                    buttons (`SideColumns`: × closes it to the offer, Don't show again, keep, Coding agents settings)
//   `plus-menu [pick <key>]`         the + menu "New terminal with": `plus-menu` then `<key> "<name>" [default]` a line; a choice
//   `comment-at <row>`               the comment dialog of the diff's row `row` (as `diff-spans` numbers it) opened, as a click
//                                    on the line opens it (the sheet: `windows` says `sheet=true`); `comment <text>` then saves it
//   `review`                         what pasting the review would paste (`Review.message`), or `review: none`
//   `title-words` / `exited-words`   what the title row says after the name, what the exited bar says ("fish exited"), or none
//   `overlay`                        the palette or the shortcut sheet up over the window: `overlay <palette|shortcuts|none>`
//   `trust-state`                    what the open project commands window shows (`trust` opens it), or none
//   `step-draft`                     Notes' Add next step: the draft row opens (`notes` says `draft=true`)
//   `project-layout` / `project-note <prompt>`  the Project tab's Open terminals, a prompt's Copy into a new note

import AppKit
import AuxModel
import AuxWindows
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
            if argument == "state" {
                // Who the list is asked for (until it closes), and whether a popover is open over the window.
                let list = window?.childWindows?.contains { String(describing: type(of: $0)).contains("Popover") } == true
                reply("branch-chip picker=\(hub.panel.changes.pickerFor.map(String.init) ?? "-") list=\(list ? "open" : "closed")")
                return true
            }
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
        case "title-words": reply("title-words " + (hub.titleWords(look: look).map(Readout.quote) ?? "none"))
        case "exited-words": reply("exited-words " + (hub.exitedWords.map(Readout.quote) ?? "none"))
        case "overlay": reply("overlay \(hub.overlayName)")
        case "step-draft":
            // Notes' "Add next step": the draft row opens at the end of the steps (`StepsCard`'s button).
            let notes = hub.panel.notes
            notes.model.stepsCollapsed = false
            (notes.draftAfter, notes.drafting) = (nil, true)
            reply("step-draft")
        case "trust-state":
            // What the open project commands window draws (its model's review, which follows the project's changes):
            // `trust-state <status> "<name>" summary="…" [problem="…"] lines="<n>"`, or `trust-state none` while it is closed.
            guard let review = TrustWindow.shown?.review else { reply("trust-state none"); return true }
            var pairs = [("summary", review.summary), ("lines", String(review.lines.count))]
            if let problem = review.problem { pairs.append(("problem", problem)) }
            reply(Readout.line("trust-state", ["\(review.status)"], quoted: [review.name], pairs: pairs))
        case "project-layout":
            // The Project tab's Open terminals (`ProjectStore.openLayout`).
            hub.panel.project.openLayout()
            reply("project-layout")
        case "project-note":
            // A prompt's Copy into a new note.
            guard let prompt = hub.panel.project.state?.prompts.first(where: { $0.id == argument }) else { reply("error: no prompt \(argument)"); return true }
            hub.panel.project.copyToNote(prompt, in: hub.panel.notes)
            reply("project-note \(argument)")
        default: return false
        }
        return true
    }

    /// The list's note after a first run without agents: the stage `SideColumns` draws and its buttons' own actions.
    private func sidebarTip(_ argument: String) -> String {
        switch argument {
        case "": return "sidebar-tip \(hub.agentsTipStage)"
        case "close": hub.closeAgentsTip()
        case "never": hub.neverAgentsTip()
        case "keep": hub.keepAgentsTip()
        case "settings": hub.actions.openSettingsAt("agents")
        default: return "error: sidebar-tip [close|never|keep|settings]"
        }
        return "sidebar-tip \(argument)"
    }

    /// The comment dialog of a row of the open Changes diff, with the comment already there if the line has one. What a click on
    /// the line does to the panel: `ChangesStore.commenting` is the one state that says "a comment is being written on line N
    /// of file F", and the diff puts its sheet up from it (and takes it down when it goes nil). Whether the sheet is up is read
    /// from the window (`windows` says `sheet=true`), never from this flag.
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
