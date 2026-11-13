// ⌘J (`insertFolderPath`): the folder picker over the terminal in front; the folder chosen is pasted at its cursor, as the
// GPUI app does (it never cd's). ⌘O (`openFolder`) is the system's panel and a new terminal there. The providers are
// core's (typed paths, the fuzzy walk, recent folders by frecency, git branch badges) where the window is real and the
// sample's elsewhere. Recent folders are kept in the profile's database (the `recent_folders` row GPUI keeps) while the
// flag is on, and a visit is recorded when a terminal changes folder (an event of the workspace's state, never a poll) and
// when the picker opens; with the flag off the stored list is cleared the first time a folder changes.

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
    private var clearedWhileOff = false
    #if WITH_UNIFFI
        private var folders: CoreFolders?
    #endif

    init(app: AppController) {
        self.app = app
        controller = FolderPickerController(policy: app.policy)
        #if WITH_UNIFFI
            if app.options.isReal {
                folders = PlatformBridge.makeFolders(recentsJSON: PlatformBridge.storedRecents())
                follow()
            }
        #endif
    }

    /// The open picker's driver and panel, for the control folder's `folder-picker` (nil while it is closed).
    var driver: FolderPickerDriver? { controller.driver }
    var panelFrame: CGRect? { controller.panel?.frame }
    func close() { controller.close() }

    /// The key's and the menu's way in: only over the terminal that has the keyboard (a paste must not land in one the
    /// person is not in) and only with the Folder picker feature flag on.
    func insertPath() {
        guard app.flagsOn(app.front).contains("folder_picker"),
            let view = app.selectedView, view.window?.isKeyWindow == true, view.window?.firstResponder === view
        else { return }
        open()
    }

    /// The picker itself, over the terminal in front; a pick is pasted there.
    func open() {
        guard let window = app.window, let content = window.contentView else { return }
        // The terminal it opens over: a pick goes only there, and the picker closes when another comes in front.
        let view = app.selectedView
        let opened = app.hub.state.selected
        let cwd = app.hub.state.terminals.first { $0.id == opened }?.folder.map { ($0 as NSString).expandingTildeInPath } ?? NSHomeDirectory()
        // Window coordinates from the top left. The anchor is the cursor cell of the last frame; before one, the pane's
        // bottom left line (GPUI `folder_picker/mod.rs`).
        func fromTop(_ frame: CGRect) -> CGRect { CGRect(x: frame.minX, y: content.bounds.height - frame.maxY, width: frame.width, height: frame.height) }
        let pane = view.map { fromTop($0.convert($0.bounds, to: nil)) } ?? content.bounds
        let cursor = view?.cursorFrameInWindow.map(fromTop) ?? CGRect(x: pane.minX, y: pane.maxY - 20, width: 10, height: 20)
        visit(cwd)
        let (providers, decorators) = sources()
        controller.open(
            over: window, pane: pane, cursor: cursor, context: PickerContext(cwd: cwd), providers: providers, decorators: decorators,
            onPick: { [weak view] path in view?.source.paste(path) }, onForget: { [weak self] path in self?.forget(path) })
        closeWhenSelectionMoves(from: opened)
    }

    private func closeWhenSelectionMoves(from opened: TerminalID?) {
        withObservationTracking { _ = app.hub.state.selected } onChange: { [weak self] in
            Task { @MainActor in
                guard let self else { return }
                if self.app.hub.state.selected != opened { self.controller.close() } else { self.closeWhenSelectionMoves(from: opened) }
            }
        }
    }

    private func sources() -> ([any FolderProviding], [any FolderDecorating]) {
        #if WITH_UNIFFI
            if let folders { return (folders.providers, folders.decorators) }
        #endif
        return ([MockFolders.sample], [MockFolders.sample])
    }

    /// The folder of the terminal the picker opens over counts as a visit (a terminal started there never said so).
    private func visit(_ folder: String) {
        #if WITH_UNIFFI
            if folders?.recordVisit(folder) == true { save() }
        #endif
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
            PlatformBridge.storeRecents(json)
        }

        /// Records each terminal's folder as it changes: every window's, folded groups included (every folder change is a visit).
        private func follow() {
            let terminals = withObservationTracking { app.sessions.flatMap { $0.hub.state.all }.map { ($0.id, $0.folder) } } onChange: { [weak self] in
                Task { @MainActor in self?.follow() }
            }
            // Kept only while the flag is on; the first change with it off forgets what it learned.
            guard app.flagsOn(app.front).contains("folder_picker") else {
                if !clearedWhileOff, terminals.contains(where: { $0.1 != nil }) {
                    clearedWhileOff = true
                    folders?.clear()
                    save()
                }
                return
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
