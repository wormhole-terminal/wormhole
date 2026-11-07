// The Notes tab's text per terminal: loaded when a terminal is shown, saved after each edit, forgotten with the terminal,
// and the row's note count told to the workspace. While the tab is up (and only then) it is merged with the trusted
// project's `notes.md` every couple of seconds: the file watch lives in Rust, and this is its slow poll.

import AppKit
import Observation
import PanelModel
import WindowUI
import WorkspaceModel

@MainActor
final class NotesCoordinator {
    private let persistence: any NotesPersisting
    private unowned let hub: WindowHub
    private var shown: TerminalID?
    private var known: Set<TerminalID> = []
    private var saved = NotesModel()
    private var syncTimer: Timer?
    private static let syncEvery: TimeInterval = 2

    init(persistence: any NotesPersisting, hub: WindowHub) {
        self.persistence = persistence
        self.hub = hub
    }

    func start() {
        persistence.idsStart(at: (hub.state.all.map(\.id).max() ?? 0) + 1)
        follow()
    }

    private func follow() {
        withObservationTracking {
            _ = (hub.state.selected, hub.state.all.count, hub.panel.notes.model, hub.panel.tab, hub.state.panelVisible)
        } onChange: { [weak self] in
            Task { @MainActor in
                self?.changed()
                self?.follow()
            }
        }
        changed()
    }

    private func changed() {
        let selected = hub.state.selected
        forgetClosed()
        if selected != shown {
            if let shown, hub.state.all.contains(where: { $0.id == shown }) { persistence.save(session: shown, model: hub.panel.notes.model) }
            shown = selected
            let loaded = selected.map { persistence.load(session: $0) } ?? NotesModel()
            saved = loaded
            if hub.panel.notes.model != loaded { hub.panel.notes.model = loaded }
        } else if let shown, hub.panel.notes.model != saved {
            saved = hub.panel.notes.model
            persistence.save(session: shown, model: saved)
            tell(shown, saved)
        }
        setSync(on: hub.panel.tab == .notes && hub.state.panelVisible)
    }

    /// Terminals moving to another window are not closed: what was written for them goes with them, not away.
    func departing(_ ids: [TerminalID]) { known.subtract(ids) }

    private func forgetClosed() {
        let now = Set(hub.state.all.map(\.id))
        for gone in known.subtracting(now) { persistence.closed(session: gone) }
        known = now
    }

    /// The row's count of what is still open.
    private func tell(_ id: TerminalID, _ model: NotesModel) {
        hub.store.hooks?.setNotes(id: id, tasksDone: model.tasks.items.map(\.done), notes: model.notes.map(\.text))
    }

    private func setSync(on: Bool) {
        guard on != (syncTimer != nil) else { return }
        syncTimer?.invalidate()
        syncTimer = on ? Timer.scheduledTimer(withTimeInterval: Self.syncEvery, repeats: true) { [weak self] _ in
            Task { @MainActor in await self?.sync() }
        } : nil
    }

    private func sync() async {
        guard let id = shown, let root = hub.panel.project.state?.root else { return }
        let editing = NSApp.keyWindow?.firstResponder is NSTextView
        guard let merged = await persistence.sync(session: id, root: root, model: hub.panel.notes.model, editing: editing),
            id == shown, merged != hub.panel.notes.model
        else { return }
        saved = merged
        hub.panel.notes.model = merged
    }
}
