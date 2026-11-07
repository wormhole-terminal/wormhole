// Where the Notes tab's text is kept, as the window needs it: loaded when a terminal is shown, saved after an edit, merged
// with the project's `notes.md` while the tab is up. The core's store (`RustNotesPersistence`) is the one implementation.

import PanelModel

protocol NotesPersisting: AnyObject, Sendable {
    func load(session: Int) -> NotesModel
    @discardableResult func save(session: Int, model: NotesModel) -> Bool
    func sync(session: Int, root: String, model: NotesModel, editing: Bool) async -> NotesModel?
    func closed(session: Int)
    func idsStart(at firstFree: Int)
}

#if WITH_UNIFFI
    import FilesBridge

    extension RustNotesPersistence: NotesPersisting {}
#endif
