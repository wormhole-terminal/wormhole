// Where the Notes tab's text lives: in the app's database (`SessionNotesStore`), and, in a trusted project that
// wants it, in the project's `notes.md` (`ProjectService.syncNotes`, core's three-way merge). `NotesStore` keeps the
// text as a value; the host loads it when a terminal opens, saves it after a change and calls `sync` on a timer
// while the Notes tab is up.

import Foundation
import PanelModel
import WormholeBindings

public final class RustNotesPersistence: @unchecked Sendable {
    private let notes: SessionNotesStore
    private let project: ProjectService

    init(notes: SessionNotesStore, project: ProjectService) {
        self.notes = notes
        self.project = project
    }

    /// What was saved for terminal `session`; empty for one never saved.
    public func load(session: Int) -> NotesModel { NotesModel(saved: notes.load(session: Int64(session))) }

    /// Saves terminal `session`'s notes and steps. Rows that did not change are not rewritten, so this can follow
    /// every edit. False when the write failed.
    @discardableResult
    public func save(session: Int, model: NotesModel) -> Bool { notes.save(session: Int64(session), saved: model.saved) }

    /// One sync step with the `notes.md` of the project at `root` (`ProjectState.root`): the model to show instead,
    /// or nil when nothing changed. `editing`: a note or step is being written, so the merge waits. Nothing is read
    /// or written while the project is not trusted (Rust decides); the Project tab says why.
    public func sync(session: Int, root: String, model: NotesModel, editing: Bool) async -> NotesModel? {
        let (project, document) = (project, model.document)
        let outcome = await offMain { project.syncNotes(session: Int64(session), root: root, local: document, editing: editing) }
        return model.applying(outcome)
    }

    /// The terminal was closed: the base it merged against goes with it.
    public func closed(session: Int) { project.notesSessionClosed(session: Int64(session)) }

    /// Call once at launch with one past the highest terminal id in use.
    public func idsStart(at firstFree: Int) { project.notesIdsStartAt(firstFree: Int64(firstFree)) }
}
