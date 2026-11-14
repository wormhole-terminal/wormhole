// What the palette lists beyond the keymap, and what ⌘↩ and the shell rows do: the terminals of every window, the front
// terminal's trusted project, a new terminal with another shell, a new window where a result is.

import AppKit
import PaletteModel
import PlatformModel
import PanelModel
import WindowUI
import WorkspaceModel

extension AppController {
    /// Every visible window's terminals, those in folded groups too, with the window's number (parked windows are left out and
    /// not counted); only the palette's own window has ⌘ keys (core `gather`).
    func paletteTerminals(own: WindowSession?) -> [PaletteTerminal] {
        sessions.filter { !$0.parked }.enumerated().flatMap { index, session in
            let rows = Dictionary(session.hub.state.sections.flatMap(\.rows).map { ($0.id, $0) }, uniquingKeysWith: { first, _ in first })
            return session.hub.state.all.map { terminal in
                let row = rows[terminal.id]
                let ambient = ambient(terminal, running: terminal.runningCommand)
                return PaletteTerminal(
                    id: terminal.id, name: ambient.name, window: index + 1, activity: terminal.activity, told: ambient.source.told,
                    number: session === own ? row?.number : nil, path: terminal.folder, folder: ambient.folder, branch: ambient.branch,
                    who: ambient.who, message: ambient.message, command: ambient.command, exit: ambient.exit,
                    elapsed: { [state = ambient.state, since = ambient.since] now in
                        AmbientWording.elapsed(state, since: max(0, now.timeIntervalSince(since)))
                    })
            }
        }
    }

    /// A new terminal that runs the shell with this key (New terminal: fish), in the window the palette or menu is over.
    func newTerminal(shellKey key: String, in session: WindowSession? = nil) {
        guard settingsModel.host.shellChoices().contains(where: { $0.key == key }) else { return }
        (session?.hub ?? hub).send(.newTerminalWithShell(key))
    }

    /// ⌘↩: a new window, laid out like this one, with a terminal where the result is: a terminal's folder, a folder, a commit's
    /// repository with History on the commit there.
    func openInNewWindow(_ target: Command.Target, from session: WindowSession?) {
        func expanded(_ path: String) -> String { HomeFolder.expanding(path) }
        let hub = (session ?? front).hub
        switch target {
        case .terminal(let id): openNewWindow(folder: self.session(holding: id)?.hub.state.all.first { $0.id == id }?.folder.map(expanded))
        case .folder(let path): openNewWindow(folder: expanded(path))
        case .commit(let sha):
            guard let repo = hub.state.all.first(where: { $0.id == hub.state.selected })?.repo else { return }
            openNewWindow(folder: repo)
            // The window opens a step later and is then the one in front; History follows its folder once git has answered.
            Task { @MainActor [weak self] in
                for _ in 0..<40 {
                    try? await Task.sleep(for: .milliseconds(50))
                    guard let opened = self?.sessions.last?.hub, opened.panel.history.phase == .loaded else { continue }
                    opened.send(.showPanelTab(.history))
                    opened.panel.history.reveal(sha)
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
