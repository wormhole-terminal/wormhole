// The control folder's commands for the right panel's own text: `note-add <text>` and `task-add <text>` as the Notes tab's
// buttons do, and `notes` to say what the tab holds (so a check can see it was saved and loaded again).

extension AppController {
    /// False when `command` is not one of these.
    func runPanel(command: String, argument: String) -> Bool {
        let notes = hub.panel.notes
        switch command {
        case "note-add": notes.add(note: argument)
        case "task-add": _ = notes.model.tasks.add(argument)
        case "notes":
            reply("notes=\(notes.model.notes.map(\.text)) steps=\(notes.model.tasks.items.map { "\($0.text):\($0.done)" })")
            return true
        case "notices":
            reply("notices=\((notices?.logged ?? []).map { "\($0.title): \($0.body)" })")
            return true
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
