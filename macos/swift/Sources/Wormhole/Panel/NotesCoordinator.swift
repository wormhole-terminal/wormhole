// The Notes tab's text per terminal: loaded when a terminal is shown, saved after each edit, forgotten with the terminal,
// and the row's note count told to the workspace. Every terminal of the window, whatever tab is up, is merged with its trusted
// project's `notes.md` every couple of seconds (`project/notes_sync.rs` runs on the tick for all of them): the file watch lives
// in Rust, and this is its slow poll. A terminal that is not shown is merged from what was saved for it.

import AppKit
import Observation
import PanelModel
import WindowUI
import WorkspaceModel

@MainActor
final class NotesCoordinator {
    private let persistence: any NotesPersisting
    /// Held, not `unowned`: a merge in flight when the window closes still reads the hub after its `await`s (the hub
    /// never names this coordinator, so nothing is kept alive in a circle).
    private let hub: WindowHub
    private var shown: TerminalID?
    private var known: Set<TerminalID> = []
    private var saved = NotesModel()
    private var syncTimer: Timer?
    private var projectOn = true
    private var syncing = false
    /// Terminals handed to another window, whose merge in flight is still stored for them.
    private var departed: Set<TerminalID> = []
    /// A folder's project root as last asked, so a window of many terminals asks each folder only now and then.
    private var roots: [String: (root: String?, at: Date)] = [:]
    private static let syncEvery: TimeInterval = 2

    init(persistence: any NotesPersisting, hub: WindowHub) {
        self.persistence = persistence
        self.hub = hub
    }

    func follow(profile id: String) { persistence.follow(profile: id) }

    func start() {
        persistence.idsStart(at: (hub.state.all.map(\.id).max() ?? 0) + 1)
        follow()
    }

    private func follow() {
        withObservationTracking {
            _ = (hub.state.selected, hub.state.all.count, hub.panel.notes.model, hub.panel.tab, hub.state.panelVisible, hub.state.layout)
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
            // A draft step belongs to the terminal it was started in (`features/notes/def.rs` `selected`).
            hub.panel.notes.leaveDraft()
            let loaded = selected.map { persistence.load(session: $0) } ?? NotesModel()
            saved = loaded
            if hub.panel.notes.model != loaded { hub.panel.notes.model = loaded }
        } else if let shown, hub.panel.notes.model != saved {
            saved = hub.panel.notes.model
            persistence.save(session: shown, model: saved)
            tell(shown, saved)
        }
        // The core's Project service is told whether project folders are on for this window; off, it reads and writes
        // no `.wormhole/`. The sync needs the notes on as well (`project/notes_sync.rs`).
        let project = offers(.project)
        if project != projectOn {
            projectOn = project
            persistence.projectFeature(on: project)
        }
        setSync(on: project && offers(.notes) && !hub.state.all.isEmpty)
    }

    /// Whether the feature behind `tab` is on for this window (core's feature mask, which the tabs are drawn from).
    private func offers(_ tab: WorkspaceModel.PanelTab) -> Bool {
        hub.state.layout.map { $0.tabs.contains { $0.tab == tab && $0.enabled } } ?? true
    }

    /// Terminals moving to another window are not closed: what was written for them goes with them, not away.
    func departing(_ ids: [TerminalID]) {
        known.subtract(ids)
        departed.formUnion(ids)
    }

    private func forgetClosed() {
        let now = Set(hub.state.all.map(\.id))
        for gone in known.subtracting(now) { persistence.closed(session: gone) }
        known = now
        // A folder's project root is asked for its terminals: with the last of them gone it is not kept.
        let folders = Set(hub.state.all.compactMap { AppController.expanded($0.folder) })
        roots = roots.filter { folders.contains($0.key) }
    }

    /// The row's count of what is still open.
    private func tell(_ id: TerminalID, _ model: NotesModel) {
        hub.store.hooks?.setNotes(id: id, tasksDone: model.tasks.items.map(\.done), notes: model.notes.map(\.text))
    }

    private func setSync(on: Bool) {
        guard on != (syncTimer != nil) else { return }
        syncTimer?.invalidate()
        syncTimer = on ? Timer.scheduledTimer(withTimeInterval: Self.syncEvery, repeats: true) { [weak self] timer in
            // The window is gone: the timer ends with it instead of firing for nobody.
            guard let self else { return timer.invalidate() }
            Task { @MainActor in await self.sync() }
        } : nil
    }

    func sync() async {
        // A second run starting before the first lands would merge against what the first is still to write.
        guard !syncing else { return }
        syncing = true
        defer { syncing = false }
        for terminal in hub.state.all {
            let id = terminal.id
            var found = id == shown ? hub.panel.project.state?.root : nil
            if found == nil { found = await root(of: terminal) }
            guard let root = found, hub.state.all.contains(where: { $0.id == id }) else { continue }
            // Only a step or note of the terminal on show being written holds its merge back (`api.rs` `being_written`);
            // read just before the merge is sent, after the awaits above.
            let editing = id == shown && hub.panel.notes.isWriting
            let sent = id == shown ? hub.panel.notes.model : persistence.load(session: id)
            let answered = await persistence.sync(session: id, root: root, model: sent, editing: editing)
            land(answered, sent: sent, for: id)
        }
    }

    /// The core has merged against `notes.md` and moved the terminal's merge base: the model kept for THAT terminal moves with
    /// it, whichever terminal is shown now, or the next merge reads the lines the file gained as lines the person deleted.
    private func land(_ answered: NotesModel?, sent: NotesModel, for id: TerminalID) {
        guard hub.state.all.contains(where: { $0.id == id }) else {
            // Closed during the merge: the base it just set goes again (a terminal that moved to another window keeps its notes,
            // and the merge is stored for it).
            if departed.contains(id) { store(answered, sent: sent, for: id) } else { persistence.closed(session: id) }
            return
        }
        store(answered, sent: sent, for: id)
    }

    private func store(_ answered: NotesModel?, sent: NotesModel, for id: TerminalID) {
        guard let answered else { return }
        let showing = id == shown
        // A tick or a new step made while the merge ran (no editor open, so not "being written") is kept on top of it.
        let now = showing ? hub.panel.notes.model : persistence.load(session: id)
        let merged = now == sent ? answered : NotesModel.rebase(merged: answered, sent: sent, now: now)
        guard merged != now else { return }
        // Saved like an edit, at once: a merge left for the next edit is lost on quit, and the next launch
        // then reads the stale database against the merged base as lines the person deleted.
        if showing {
            hub.panel.notes.model = merged
            saved = merged
        }
        persistence.save(session: id, model: merged)
        if hub.state.all.contains(where: { $0.id == id }) { tell(id, merged) }
    }

    private func root(of terminal: TerminalSummary) async -> String? {
        guard let folder = AppController.expanded(terminal.folder) else { return nil }
        if let known = roots[folder], Date().timeIntervalSince(known.at) < 10 { return known.root }
        let root = await hub.panel.project.root(of: folder)
        roots[folder] = (root, Date())
        return root
    }
}
