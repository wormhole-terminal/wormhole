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
        case "profile-new":
            reply("profile-new \(argument) \(settingsModel.host.makeProfile(name: argument, copyOf: nil) ?? "ok")")
            return true
        case "profile-switch":
            switchProfile(to: argument)
            reply("profile-switch \(argument) now=\(windowsHost.profile) shown=\(shown.map(\.id)) parked=\(sessions.filter(\.parked).map(\.id))")
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
        case "full-screen", "window-zoom":
            // `full-screen ID` and `window-zoom ID` do what the window's green light does (full screen, or zoom to the display).
            guard let id = numbers.first, let window = sessions.first(where: { $0.id == id })?.window else {
                reply("error: \(command) needs the id of a window")
                return true
            }
            if command == "full-screen" { window.toggleFullScreen(nil) } else { window.zoom(nil) }
        case "minimise", "place":
            // `minimise ID` puts the window in the Dock; `place ID X Y W H` puts it there (the core's space: y down).
            guard let id = numbers.first, let window = sessions.first(where: { $0.id == id })?.window else {
                reply("error: \(command) needs the id of a window")
                return true
            }
            if command == "minimise" {
                window.miniaturize(nil)
            } else if numbers.count == 5 {
                WindowFrames.place(WindowFrame(x: Double(numbers[1]), y: Double(numbers[2]), width: Double(numbers[3]), height: Double(numbers[4]), display: WindowFrames.displays().first?.id), on: window)
            }
        case "profile-menu":
            // The Profile menu's "Open Window in" / "Move Window to" by the row's place in the list: `profile-menu open 1`.
            guard let index = numbers.first, let kind = argument.split(separator: " ").first else {
                reply("error: profile-menu open|move INDEX")
                return true
            }
            router.dispatch(kind == "open" ? .openInProfile(index) : .moveToProfile(index))
            reply("profile-menu \(argument)")
            return true
        case "tear-off":
            // As the sidebar's gesture lets a terminal go: `tear-off ID X Y` at the window's own space (the front window's),
            // `tear-off-at ID X Y` at the primary display's (from its top left, y down) with nothing covering it.
            guard numbers.count == 3, let source = session(holding: numbers[0]) else {
                reply("error: \(command) needs a terminal id and a point")
                return true
            }
            let point = CGPoint(x: CGFloat(numbers[1]), y: CGFloat(numbers[2]))
            reply("tear-off \(numbers[0]) took=\(source.hub.terminalDrag.tearOff?(numbers[0], point) == true)")
            return true
        case "tear-off-at":
            guard numbers.count == 3, let source = session(holding: numbers[0]), let height = NSScreen.screens.first?.frame.height else {
                reply("error: \(command) needs a terminal id and a point")
                return true
            }
            land(numbers[0], from: source, at: CGPoint(x: CGFloat(numbers[1]), y: height - CGFloat(numbers[2])), covered: false)
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
            // Where each terminal's program runs, and what its host's bar says (nothing while it is fine).
            let store = session.hub.store
            let hosts = session.hub.state.live.map { id in
                "\(id):\(store.hosting(of: id).map { String(describing: $0) } ?? "off")\(store.hostProblem(of: id).map { " problem=\($0)" } ?? "")"
            }.joined(separator: ", ")
            return "window=\(session.id) profile=\(session.windows.profile) visible=\(session.window?.isVisible ?? false) front=\(session === front) terminals=[\(ids)] selected=\(session.hub.state.selected.map(String.init) ?? "none") frame=\(frame) hosting=[\(hosts)]"
        }.joined(separator: "\n")
    }

    private func describe(_ frame: WindowFrame) -> String {
        "\(frame.kind) \(Int(frame.x)),\(Int(frame.y)) \(Int(frame.width))x\(Int(frame.height))\(frame.minimised ? " minimised" : "")"
    }
}
