// What the palette lists beyond the keymap, and what ⌘↩ and the shell rows do: the terminals of every window, the front
// terminal's trusted project, a new terminal with another shell, a new window where a result is.

import AppKit
import PaletteModel
import PlatformModel
import PanelModel
import WindowUI
import WorkspaceModel

extension AppController {
    /// Every window's terminals with the window's number; only the palette's own window has ⌘ keys (core `gather`).
    func paletteTerminals(own: WindowSession?) -> [PaletteTerminal] {
        sessions.enumerated().flatMap { index, session in
            session.hub.state.sections.flatMap(\.rows).map { row in
                let ambient = ambient(row)
                return PaletteTerminal(
                    id: row.id, name: ambient.name, window: index + 1, activity: row.terminal.activity,
                    number: session === own ? row.number : nil, path: row.terminal.folder, folder: ambient.folder, branch: ambient.branch,
                    who: ambient.who, message: ambient.message, command: ambient.command, exit: ambient.exit,
                    elapsed: { [state = ambient.state, since = ambient.since] now in
                        AmbientWording.elapsed(state, since: max(0, now.timeIntervalSince(since)))
                    })
            }
        }
    }

    /// A new terminal that runs the shell with this key (New terminal: fish), in the window the palette or menu is over.
    func newTerminal(shellKey key: String, in session: WindowSession? = nil) {
        guard let shell = settingsModel.host.shellChoices().first(where: { $0.key == key }) else { return }
        (session?.hub ?? hub).send(.newTerminalRunning(program: shell.program, arguments: shell.arguments))
    }

    /// ⌘↩: a new window, laid out like this one, with a terminal where the result is: a terminal's folder, a folder, a commit's
    /// repository with History on the commit there.
    func openInNewWindow(_ target: Command.Target, from session: WindowSession?) {
        func expanded(_ path: String) -> String { (path as NSString).expandingTildeInPath }
        let hub = (session ?? front).hub
        switch target {
        case .terminal(let id): openNewWindow(folder: self.session(holding: id)?.hub.state.all.first { $0.id == id }?.folder.map(expanded))
        case .folder(let path): openNewWindow(folder: expanded(path))
        case .commit(let sha):
            guard let repo = hub.state.all.first(where: { $0.id == hub.state.selected })?.repo else { return }
            openNewWindow(folder: repo)
            // The window opens a step later and is then the one in front; History lists its commits once git has answered.
            Task { @MainActor [weak self] in
                for _ in 0..<40 {
                    try? await Task.sleep(for: .milliseconds(50))
                    guard let opened = self?.sessions.last?.hub, opened.panel.history.model.commits.contains(where: { $0.sha == sha }) else { continue }
                    opened.send(.showPanelTab(.history))
                    opened.panel.history.show(sha)
                    return
                }
            }
        default: break
        }
    }
}

extension PaletteProject {
    /// The project of the terminal in front, once it is trusted with Actions on (the core leaves the actions out otherwise).
    init?(_ state: ProjectState?) {
        guard let state, !state.actions.isEmpty else { return nil }
        self.init(
            name: "\(state.name)/.wormhole",
            actions: state.actions.map { PaletteProject.Action(id: $0.id, title: $0.title, detail: $0.detail, key: $0.key) })
    }
}
