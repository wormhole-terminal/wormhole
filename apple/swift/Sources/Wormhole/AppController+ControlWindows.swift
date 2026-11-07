// The control folder's commands for several windows (`WORMHOLE_DEVCTL`): list them, open one, make another the one commands
// act on, move terminals between windows, write the saved records now and read back what was saved.

import AppKit
import Foundation
import WindowUI
import WindowsModel
import WorkspaceModel

extension AppController {
    /// False when `command` is not one of these.
    func runWindows(command: String, argument: String) -> Bool {
        let numbers = argument.split(separator: " ").compactMap { Int($0) }
        switch command {
        case "new-window": openNewWindow(folder: argument.isEmpty ? nil : argument)
        case "sessions":
            reply(sessionLines())
            return true
        case "use-window":
            guard let id = numbers.first, let session = sessions.first(where: { $0.id == id }) else {
                reply("error: use-window needs the id of a window (see sessions)")
                return true
            }
            lastFront = session
            menus?.windowsChanged()
        case "move-to-window":
            guard let first = numbers.first, let source = session(holding: first) else {
                reply("error: move-to-window needs the ids of terminals of one window")
                return true
            }
            guard moveToNewWindow(numbers, from: source) else { reply("error: nothing moved (this workspace cannot hand terminals over)"); return true }
        case "save-windows":
            saveAllWindows()
        case "saved-windows":
            let lines = windowsHost.backend.saved().map { "saved window=\($0.id) terminals=\($0.terminals) frame=\($0.frame.map(describe) ?? "none")" }
            reply(lines.isEmpty ? "saved none" : lines.joined(separator: "\n"))
            return true
        default: return false
        }
        reply(argument.isEmpty ? command : "\(command) \(argument)")
        return true
    }

    /// One line per window: its id, terminals, where it is and whether commands act on it.
    private func sessionLines() -> String {
        sessions.map { session in
            let frame = frame(of: session).map(describe) ?? "none"
            let ids = session.hub.state.live.map(String.init).joined(separator: ",")
            return "window=\(session.id) front=\(session === front) terminals=[\(ids)] selected=\(session.hub.state.selected.map(String.init) ?? "none") frame=\(frame)"
        }.joined(separator: "\n")
    }

    private func describe(_ frame: WindowFrame) -> String {
        "\(frame.kind) \(Int(frame.x)),\(Int(frame.y)) \(Int(frame.width))x\(Int(frame.height))\(frame.minimised ? " minimised" : "")"
    }
}
