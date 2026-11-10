// The control folder's commands for the right panel's own text: `note-add <text>` and `task-add <text>` as the Notes tab's
// buttons do, and `notes` to say what the tab holds (so a check can see it was saved and loaded again). `history N`
// opens the Nth commit of the log (0 folds it), `history-file N` the Nth file of that commit, `history-rev <name>` the branch picker, `history-copy N <what>`, `history-state` and
// `project-state` say what the tabs hold, `project-approve`, `project-feature <key> on|off`, `project-run <id>` and
// `project-send <id>` are the Project tab's buttons.

import PanelModel
import PanelUI
import PlatformModel

extension AppController {
    private func fail(_ message: String) -> Bool {
        reply("error: \(message)")
        return true
    }

    /// False when `command` is not one of these.
    func runPanel(command: String, argument: String) -> Bool {
        let notes = hub.panel.notes
        switch command {
        case "note-add": notes.add(note: argument)
        case "task-add": _ = notes.model.tasks.add(argument)
        case "notes":
            reply("notes=\(notes.model.notes.map(\.text)) steps=\(notes.model.tasks.items.map { "\($0.text):\($0.done)" })")
            return true
        case "history":
            hub.send(.showPanelTab(.history))
            let history = hub.panel.history
            if let n = Int(argument), n > 0, history.model.commits.indices.contains(n - 1) {
                if history.expanded != history.model.commits[n - 1].id { history.toggle(history.model.commits[n - 1].id) }
            } else if let open = history.expanded {
                history.toggle(open)
            }
        case "history-file":
            let history = hub.panel.history
            guard let n = Int(argument), history.files.indices.contains(n - 1) else { return fail("history-file needs the number of a file of the open commit") }
            history.open(history.files[n - 1].id)
        case "history-rev":
            // The branch picker's choice: a name, `stashes`, or nothing for HEAD.
            hub.panel.history.pick(argument.isEmpty ? nil : argument == "stashes" ? .stashes : Rev(name: argument))
        case "history-copy":
            // What a copy of the Nth commit puts on the clipboard: hash, shortHash, message, subject or author.
            let history = hub.panel.history
            let parts = argument.split(separator: " ").map(String.init)
            guard parts.count == 2, let n = Int(parts[0]), history.model.commits.indices.contains(n - 1),
                let what = CommitCopy.allCases.first(where: { "\($0)" == parts[1] })
            else { return fail("history-copy N hash|shortHash|message|subject|author") }
            let commit = history.model.commits[n - 1]
            Task { reply("copy=\(await history.text(of: what, for: commit) ?? "nil")") }
            return true
        case "history-state":
            let history = hub.panel.history, details = history.details
            reply("phase=\(history.phase) commits=\(history.model.commits.count) branch=\(history.model.branch ?? "-") open=\(history.expanded.map { String($0.prefix(7)) } ?? "-") "
                + "people=\(details?.people.map { "\($0.label)=\($0.spans.map(\.text).joined())" } ?? []) parents=\(details?.parents.map(\.short) ?? []) "
                + "changes=\(details?.changes.spans.map(\.text).joined() ?? "-") files=\(history.files.map(\.path)) diff=\(hub.panel.history.diff.phase)")
            return true
        case "project-approve": hub.panel.project.approve()
        case "project-feature":
            let words = argument.split(separator: " ").map(String.init)
            guard words.count == 2, let feature = ProjectFeature(rawValue: words[0]) else { return fail("project-feature <settings|theme|actions|prompts|layout|notes> on|off") }
            hub.panel.project.setFeature(feature, on: words[1] == "on")
        case "project-run", "project-send":
            let project = hub.panel.project
            if command == "project-run", let action = project.state?.actions.first(where: { $0.id == argument }) { project.run(action) }
            else if let prompt = project.state?.prompts.first(where: { $0.id == argument }) { project.send(prompt) }
            else { return fail("\(command): no such id \(argument)") }
        case "project-state":
            let project = hub.panel.project
            guard let state = project.state else { reply("project=none loaded=\(project.loaded)"); return true }
            reply("root=\(state.root) status=\(state.status) asks=\(state.asksForTrust) commands=\(state.commands.count) features=\(state.features.map { "\($0.feature.rawValue)=\($0.on)" }) "
                + "actions=\(state.actions.map(\.id)) prompts=\(state.prompts.map(\.id)) layout=\(state.layout) errors=\(state.errors)")
            return true
        case "panel-option":
            // The settings the panel reads, set as the settings follower would: tree|group|split|wrap on|off, tabwidth N.
            let preferences = hub.panel.preferences
            let words = argument.split(separator: " ").map(String.init)
            let on = words.last == "on"
            switch words.first {
            case "tree": preferences.changesTree = on
            case "group": preferences.groupUntracked = on
            case "split": preferences.diffSplit = on
            case "wrap": preferences.diffWrap = on
            case "tabwidth": preferences.tabWidth = words.last.flatMap(Int.init) ?? 4
            default: return fail("panel-option tree|group|split|wrap on|off, or tabwidth N")
            }
            reply("tree=\(preferences.changesTree) group=\(preferences.groupUntracked) split=\(preferences.diffSplit) wrap=\(preferences.diffWrap) tabwidth=\(preferences.tabWidth)")
            return true
        case "files-open", "files-expand", "files-rename-start":
            // The Files tab by relative path: the viewer, a folder opened (it and the ones above), a row's name as its rename field.
            hub.send(.showPanelTab(.files))
            let files = hub.panel.files
            switch command {
            case "files-open": files.view(argument)
            case "files-expand":
                var path = ""
                for part in argument.split(separator: "/") {
                    path += (path.isEmpty ? "" : "/") + part
                    if let row = files.rows.first(where: { $0.path == path }), case .folder(open: false) = row.kind { files.toggle(path) }
                }
            default:
                guard let row = files.rows.first(where: { $0.path == argument }) else { return fail("files-rename-start: no row \(argument)") }
                files.startRename(row)
            }
        case "notices":
            reply("notices=\((notices?.logged ?? []).map { "\($0.title): \($0.body)" })")
            return true
        case "notice-log":
            // Each banner as the centre would be handed it: id, thread, level, sound, category.
            reply((notices?.logged ?? []).map { "\($0.id) thread=\($0.thread) level=\($0.level) sound=\($0.sound) category=\($0.category.rawValue) title=\($0.title) body=\($0.body)" }.joined(separator: "\n"))
            return true
        case "notice-respond":
            let words = argument.split(separator: " ", maxSplits: 2).map(String.init)
            guard words.count >= 2 else { return fail("notice-respond <banner id> <default|open|reply|show-changes> [reply text]") }
            notices?.respond(NoticeResponse(id: words[0], action: words[1] == "default" ? nil : words[1], reply: words.count > 2 ? words[2] : nil))
        case "notice-tabs":
            #if WITH_UNIFFI
                reply("notice-tabs=\(noticeTabs.map { "\($0.tab):\($0.state) \($0.source) selected=\($0.selected) key=\($0.key) since=\($0.sinceMs)" })")
            #endif
            return true
        case "notice-test":
            notices?.sendTest(name: "Wormhole", settings: settingsNotice)
        default: return false
        }
        reply("\(command) \(argument)")
        return true
    }
}
