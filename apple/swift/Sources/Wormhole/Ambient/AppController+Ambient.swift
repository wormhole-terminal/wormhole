// What choosing a row of the menu bar extra, the Dock's menu or the quick panel does (`status_item::run_command`): the app's, so a
// test instance, which makes none of those surfaces, can run the same commands (`ambient-run`). Open Folder… asks for a folder and
// opens a terminal in it; a project opens its terminal, the one already in its folder when there is one; an action runs in that
// terminal, in a window of the profile shown.

import AppKit
import PanelModel
import PlatformModel
import WorkspaceModel

extension AppController {
    func run(ambient command: AmbientCommand) {
        switch command {
        case .showApp: if policy.touchesSystem { NSApp.activate() }
        case .newTerminal: hub.send(.newTerminal)
        case .newWindow: openNewWindow()
        case .openTab(let tab): show(terminal: Int(tab))
        case .reply(let tab, let text): session(holding: Int(tab))?.hub.terminals.existing(Int(tab))?.source.write(Data((text + "\r").utf8))
        case .jumpToWaiting: hub.goTo(.needsInput)
        case .openSettings: showSettings()
        case .checkForUpdates: aux.showUpdates()
        case .quit: NSApp.terminate(nil)
        case .openFolder: openFolder()
        case .openProject(let path): openProject(path)
        case .runAction(let root, let id): runAction(root: root, id: id)
        }
    }

    /// ⌘O, the palette's Open folder… and the status item's: a terminal in a folder chosen in the system's panel, which is
    /// shown without blocking. A test instance shows no panel: it waits for the control folder's `open-folder <path>`.
    func openFolder() {
        let start: (String) -> Void = { [weak self] folder in self?.hub.send(.newTerminalIn(folder)) }
        guard policy.touchesSystem else { return pendingFolder = start }
        let panel = NSOpenPanel()
        panel.canChooseFiles = false
        panel.canChooseDirectories = true
        panel.allowsMultipleSelection = false
        panel.prompt = "Open"
        panel.begin { result in
            guard result == .OK, let folder = panel.url?.path else { return }
            MainActor.assumeIsolated { start(folder) }
        }
    }

    func openProject(_ root: String) {
        if let id = terminal(in: root, shownProfileOnly: false) { show(terminal: id) } else { hub.send(.newTerminalIn(root)) }
    }

    func runAction(root: String, id: String) {
        guard let terminal = terminal(in: root, shownProfileOnly: true), let holder = session(holding: terminal) else { return }
        show(terminal: terminal)
        // Straight to the project's own backend, at the folder asked for: whatever the Project tab shows now is not
        // consulted, and Rust refuses an untrusted project.
        let backend = holder.panelBackends.project, folder = Self.resolved(root)
        Task { await backend.run(action: id, at: folder) }
    }

    private static func resolved(_ path: String) -> String {
        URL(fileURLWithPath: (path as NSString).expandingTildeInPath).resolvingSymlinksInPath().path
    }

    /// The terminal whose folder is `root` (the core words it `~`-shortened, and /tmp is /private/tmp), in any window or only the
    /// profile shown.
    private func terminal(in root: String, shownProfileOnly: Bool) -> TerminalID? {
        let wanted = Self.resolved(root)
        for session in shownProfileOnly ? shown : sessions {
            for section in session.hub.state.sections {
                if let row = section.rows.first(where: { $0.terminal.folder.map(Self.resolved) == wanted }) { return row.id }
            }
        }
        return nil
    }
}
