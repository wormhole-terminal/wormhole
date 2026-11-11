// ⌘J (`insertFolderPath`): the folder picker over the terminal in front; the folder chosen is pasted at its cursor, as the
// GPUI app does (it never cd's). ⌘O (`openFolder`) is the system's panel and a new terminal there. The providers are
// core's (typed paths, the fuzzy walk, recent folders by frecency, git branch badges) where the window is real and the
// sample's elsewhere. Recent folders are kept in the data folder and a visit is recorded when a terminal changes folder
// (an event of the workspace's state, never a poll).

import AppKit
import Observation
import PlatformModel
import PlatformUI
import TerminalMetal
import WorkspaceModel

#if WITH_UNIFFI
    import PlatformBridge
#endif

@MainActor
final class FolderPicking {
    private let controller: FolderPickerController
    private unowned let app: AppController
    private var visited: [TerminalID: String] = [:]
    private var recents: URL { app.dataDirectory.appendingPathComponent("recent-folders.json") }
    #if WITH_UNIFFI
        private var folders: CoreFolders?
    #endif

    init(app: AppController) {
        self.app = app
        controller = FolderPickerController(policy: app.policy)
        #if WITH_UNIFFI
            if app.options.isReal {
                folders = PlatformBridge.makeFolders(recentsJSON: try? String(contentsOf: recents, encoding: .utf8))
                follow()
            }
        #endif
    }

    /// The key's and the menu's way in: only over the terminal that has the keyboard (a paste must not land in one the
    /// person is not in) and only with the Folder picker feature flag on.
    func insertPath() {
        guard app.flagsOn(app.front).contains("folder_picker"),
            let view = app.selectedView, view.window?.isKeyWindow == true, view.window?.firstResponder === view
        else { return }
        open()
    }

    /// ⌘O: a terminal in a folder chosen in the system's panel.
    func openFolder() {
        let panel = NSOpenPanel()
        panel.canChooseFiles = false
        panel.canChooseDirectories = true
        panel.allowsMultipleSelection = false
        panel.prompt = "Open"
        guard panel.runModal() == .OK, let url = panel.url else { return }
        app.hub.send(.newTerminalIn(url.path))
    }

    /// The picker itself, over the terminal in front; a pick is pasted there.
    func open() {
        guard let window = app.window, let content = window.contentView else { return }
        let cwd = app.hub.state.terminals.first { $0.id == app.hub.state.selected }?.folder.map { ($0 as NSString).expandingTildeInPath } ?? NSHomeDirectory()
        let pane = app.selectedView.map { view in
            let frame = view.convert(view.bounds, to: nil)
            return CGRect(x: frame.minX, y: content.bounds.height - frame.maxY, width: frame.width, height: frame.height)
        } ?? content.bounds
        let cursor = CGRect(x: pane.minX + 16, y: pane.minY + pane.height * 0.4, width: 10, height: 20)
        let (providers, decorators) = sources()
        controller.open(
            over: window, pane: pane, cursor: cursor, context: PickerContext(cwd: cwd), providers: providers, decorators: decorators,
            onPick: { [weak app] path in app?.selectedView?.source.paste(path) }, onForget: { [weak self] path in self?.forget(path) })
    }

    private func sources() -> ([any FolderProviding], [any FolderDecorating]) {
        #if WITH_UNIFFI
            if let folders { return (folders.providers, folders.decorators) }
        #endif
        return ([MockFolders.sample], [MockFolders.sample])
    }

    private func forget(_ path: String) {
        #if WITH_UNIFFI
            folders?.forget(path)
            save()
        #endif
    }

    #if WITH_UNIFFI
        private func save() {
            guard let json = folders?.recentsJSON() else { return }
            try? json.write(to: recents, atomically: true, encoding: .utf8)
        }

        /// Records each terminal's folder as it changes.
        private func follow() {
            let terminals = withObservationTracking { app.hub.state.terminals.map { ($0.id, $0.folder) } } onChange: { [weak self] in
                Task { @MainActor in self?.follow() }
            }
            var changed = false
            for (id, folder) in terminals {
                guard let folder, visited[id] != folder else { continue }
                visited[id] = folder
                changed = folders?.recordVisit((folder as NSString).expandingTildeInPath) == true || changed
            }
            if changed { save() }
        }
    #endif
}
