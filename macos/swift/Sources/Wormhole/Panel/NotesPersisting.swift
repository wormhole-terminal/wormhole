// Where the Notes tab's text is kept, as the window needs it: loaded when a terminal is shown, saved after an edit, merged
// with the project's `notes.md` while the tab is up. The core's store (`RustNotesPersistence`) is the one implementation.

import PanelModel

protocol NotesPersisting: AnyObject, Sendable {
    /// What was saved for a terminal (empty when nothing was), nil when the database could not be read. A load makes every
    /// item anew, with a new id (`NotesMapping.swift`).
    func read(session: Int) -> NotesModel?
    @discardableResult func save(session: Int, model: NotesModel) -> Bool
    func sync(session: Int, root: String, model: NotesModel, editing: Bool) async -> NotesModel?
    func closed(session: Int)
    func idsStart(at firstFree: Int)
    /// The window moved to profile `id`: its notes are kept there from now on.
    func follow(profile id: String)
    /// Project folders are on or off for the window: off, the core reads and writes no `.wormhole/`.
    func projectFeature(on: Bool)
}

extension NotesPersisting {
    func projectFeature(on: Bool) {}
}

#if WITH_UNIFFI
    import FilesBridge

    extension RustNotesPersistence: NotesPersisting {}
#endif
