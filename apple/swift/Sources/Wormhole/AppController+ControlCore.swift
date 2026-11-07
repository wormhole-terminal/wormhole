// What the core feeds the window, from the control folder: the columns and the title row it listed, an agent's hook as a
// hook script would send it (the hook socket is not served here yet) and the window's front/behind state.

import Foundation
import WorkspaceModel

extension AppController {
    func runCore(command: String, argument: String) -> Bool {
        switch command {
        case "columns":
            reply(hub.state.layout.map { "\($0)" } ?? "no layout from the core (sample)")
        case "title":
            reply(hub.state.title.map { "\($0)" } ?? "no title from the core (sample)")
        case "counts":
            reply("\(hub.state.counts) all=\(hub.state.all.map { "\($0.id):\($0.activity)" })")
        case "hook":
            let words = argument.split(separator: " ", maxSplits: 3).map(String.init)
            guard words.count >= 3, let id = Int(words[0]), let sink = hub.store.hooks else {
                reply("error: usage hook <terminal> <agent> <event> [message]")
                return true
            }
            sink.hook(id: id, agent: words[1], event: words[2], message: words.count > 3 ? words[3] : nil)
            reply("hook \(argument)")
        case "window-state":
            let flags = argument.split(separator: " ").map { $0 == "on" }
            guard flags.count == 2 else { reply("error: usage window-state <active on|off> <seen on|off>"); return true }
            hub.store.windowState(active: flags[0], seen: flags[1])
            reply("window-state active=\(flags[0]) seen=\(flags[1])")
        default: return false
        }
        return true
    }
}
