// The Edit menu's Undo and Redo while the Studio is the window in front: the document's own steps, named ("Undo Change
// “primary”"), as `publish_edit_titles` does in the GPUI app. The window hands this out as its undo manager, so the system's
// Edit ▸ Undo and ⌘Z reach it through the responder chain when nothing in a text field has an edit of its own to undo.

import AppKit
import StudioModel

@MainActor
final class StudioUndoManager: UndoManager {
    private let document: () -> StudioDocument

    init(document: @escaping () -> StudioDocument) {
        self.document = document
        super.init()
    }

    override var canUndo: Bool { document().undoLabel != nil }
    override var canRedo: Bool { document().redoLabel != nil }
    override var undoMenuItemTitle: String { StudioWords.undo(document().undoLabel) }
    override var redoMenuItemTitle: String { StudioWords.redo(document().redoLabel) }
    override func undo() { document().undo() }
    override func redo() { document().redo() }
}
