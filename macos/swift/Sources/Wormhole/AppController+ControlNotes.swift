// The control folder's Notes, Changes and History commands that GPUI's devctl has and the first Swift control line had not
// (`devctl/panels.rs`): `note-seed`, `note-toggle`, `task-seed`, `task-done`, `send`, `send-submit`, `stash`, `comment`,
// `compare`, `picker`, `commit-details`, `commit-more`, `project-init`. Each does what the tab's own button or key does, through
// the same stores (`NotesStore`, `ChangesStore`, `HistoryStore`, `ProjectStore`). Numbers are 1-based, as GPUI's `nth`.

import Foundation
import PanelModel
import PanelUI
import WorkspaceModel

extension AppController {
    /// False when `command` is not one of these.
    func runNotes(command: String, argument: String) async -> Bool {
        let argument = argument.trimmingCharacters(in: .whitespaces)
        let numbers = argument.split(separator: " ").compactMap { Int($0) }
        let notes = hub.panel.notes
        switch command {
        case "note-seed":
            // `note-seed <count> <lines>`: that many notes of that many lines (GPUI's text), for measuring a long list.
            let (count, lines) = (numbers.first ?? 50, numbers.dropFirst().first ?? 12)
            for n in 0..<count {
                notes.add(note: (0..<lines).map { $0 % 5 == 4 ? "" : "Note \(n) line \($0): some text that is long enough to wrap in a narrow panel." }.joined(separator: "\n"))
            }
            reply("note-seed \(count) \(lines)")
        case "note-toggle":
            guard let n = numbers.first, notes.model.notes.indices.contains(n - 1) else { reply("error: no note \(argument)"); return true }
            notes.model.notes[n - 1].collapsed.toggle()
            reply("note-toggle \(n)")
        case "task-seed":
            // `task-seed <open> <done>`: that many next steps, the done ones spread among the open ones.
            let (open, done) = (numbers.first ?? 38, numbers.dropFirst().first ?? 12)
            let total = open + done
            let ticked = Set((0..<done).map { total == 0 ? 0 : $0 * total / max(done, 1) })
            for index in 0..<total {
                guard let task = notes.model.tasks.add("Step \(index + 1)") else { continue }
                if ticked.contains(index) { notes.model.tasks.toggle(task.id) }
            }
            reply("task-seed \(open) \(done)")
        case "task-done":
            let items = notes.model.tasks.items
            guard let n = numbers.first, items.indices.contains(n - 1) else { reply("error: no next step \(argument)"); return true }
            notes.model.tasks.toggle(items[n - 1].id)
            reply("task-done \(n)")
        case "send", "send-submit":
            // The last focused note (the one the walker is on, else the newest) goes to the terminal; ⇧ presses Return after it.
            let focused = notes.cursor.flatMap { $0.hasPrefix("note-") ? UUID(uuidString: String($0.dropFirst(5))) : nil }
            guard let note = notes.model.notes.first(where: { $0.id == focused }) ?? notes.model.notes.last else { reply("error: no note to send"); return true }
            notes.send(text: String(note.text.split(separator: "\n\n", maxSplits: 1, omittingEmptySubsequences: true).first ?? ""), submit: command == "send-submit")
            reply(command)
        case "stash":
            reply(notes.stashPrompt() ? "stash: the prompt is in a new note" : "stash: nothing is typed at a prompt")
        case "comment":
            // The comment being written (a click on a line, `comment-at`): its text set (none: as typed), saved and closed, as
            // its Add Comment does. The dialog is a sheet on the window, put up from `commenting`: without the sheet on screen
            // there is nothing to write in.
            let changes = hub.panel.changes
            guard changes.commenting != nil, window?.attachedSheet != nil else { reply("error: no comment dialog is open"); return true }
            if !argument.isEmpty { changes.commenting?.text = argument }
            changes.saveComment()
            reply(changes.review.summary)
        case "compare":
            hub.panel.changes.setCompare(argument.isEmpty || argument == "HEAD" ? nil : Rev(name: argument))
            reply("compare \(argument.isEmpty ? "HEAD" : argument)")
        case "picker":
            reply(await pickerListing(argument))
        case "commit-details": reply(commitDetails())
        case "commit-more":
            hub.panel.history.bodyWhole.toggle()
            reply("commit-more \(hub.panel.history.bodyWhole ? "whole" : "compact")")
        case "notes-sync": reply(await syncNotesNow())
        case "project-init":
            let project = hub.panel.project
            // The folder the tab's button makes it in (a repository's root, as the GPUI app's `active_folder`), not the terminal's own.
            guard let folder = project.target else { reply("error: the terminal has no folder"); return true }
            project.create()
            reply("creating \(folder)/.wormhole")
        default: return false
        }
        return true
    }

    /// `notes-sync`: one whole merge of the window's terminals with their projects' notes.md, now (what the timer runs); one
    /// already under way lands first.
    private func syncNotesNow() async -> String {
        guard let coordinator = front.notes else { return "error: the window has no notes sync" }
        var ran = await coordinator.sync()
        for _ in 0..<100 where !ran {
            await pause(50)
            ran = await coordinator.sync()
        }
        return "notes-sync \(ran ? "done" : "busy")"
    }

    /// `picker [history|compare] <query>`: the revisions the picker offers for what is typed in its box, one a line.
    private func pickerListing(_ argument: String) async -> String {
        let words = argument.split(separator: " ", maxSplits: 1).map(String.init)
        let history = words.first == "history"
        let query = (history ? words.dropFirst() : words[...]).joined(separator: " ")
        hub.send(.showPanelTab(history ? .history : .changes))
        let revs: () -> [Rev] = { history ? self.hub.panel.history.revs : self.hub.panel.changes.revs }
        let loading: () -> Bool = { history ? self.hub.panel.history.revsLoading : self.hub.panel.changes.revsLoading }
        if history { hub.panel.history.loadRevs() } else { hub.panel.changes.loadRevs() }
        for _ in 0..<60 where loading() { await pause(50) }
        let shown = revs().filter { query.isEmpty || $0.name.localizedCaseInsensitiveContains(query) }
        return shown.isEmpty ? "picker: nothing" : shown.map { "\($0.kind.rawValue) \($0.name)\($0.detail.isEmpty ? "" : " \($0.detail)")" }.joined(separator: "\n")
    }

    /// `commit-details`: the open commit's block as text, one fact a line (`none` when no commit is open).
    private func commitDetails() -> String {
        let history = hub.panel.history
        guard let sha = history.expanded else { return "none" }
        var lines = ["commit \(sha)"]
        if let error = history.detailsError { lines.append("failed: \(error)") }
        guard let details = history.details else { return (lines + (history.detailsError == nil ? ["loading"] : [])).joined(separator: "\n") }
        let compact = history.compact(lineChars: 40)
        lines.append("subject: \(details.subject)")
        lines.append("message: \(history.bodyWhole ? "whole" : "compact") (toggle \(compact?.more == true ? "offered" : "none"); compact at 40 chars: \(compact?.text.debugDescription ?? "-"))")
        lines.append("body: \(details.whole.replacingOccurrences(of: "\n", with: "\\n"))")
        lines += details.people.map { "\($0.label.lowercased()): \($0.spans.map(\.text).joined())" }
        lines.append("parents: \(details.parents.map(\.sha).joined(separator: " "))")
        lines.append("refs: \(details.refs.map { "\($0.kind) \($0.name)" }.joined(separator: ", "))")
        lines.append("message cut: \(details.tooLong != nil)")
        lines.append("changes: \(details.changes.spans.map(\.text).joined())")
        return lines.joined(separator: "\n")
    }
}
