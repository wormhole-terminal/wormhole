// The Notes card: its head (the notes with something in them, and a + that starts one) and the notes. A new
// note is empty and opens for writing at once.

import DesignSystem
import PanelModel
import SwiftUI

struct NotesCard: View {
    @Bindable var store: NotesStore
    @State private var dragging: UUID?
    @Environment(\.panelMotion) private var motion

    private var model: NotesModel {
        get { store.model }
        nonmutating set { store.model = newValue }
    }

    var body: some View {
        PanelCard {
            CardBand(
                title: panelText("notes-title", "Notes"), folded: model.notes.isEmpty ? nil : model.notesCollapsed,
                count: model.filledNotes > 0 ? CountText.grouped(model.filledNotes) : nil,
                spoken: panelText("notes-spoken", ["count": .whole(model.filledNotes)], model.filledNotes == 0 ? "Notes, no notes" : "Notes, \(model.filledNotes) \(model.filledNotes == 1 ? "note" : "notes")"), onToggle: { model.notesCollapsed.toggle() },
                add: (panelText("notes-new", "New note"), { store.newNote() }))
                .walkRow("band-notes")
            if !model.notesCollapsed {
                ForEach($store.model.notes) { $note in
                    let ids = model.notes.map(\.id)
                    NoteRowView(
                        note: $note, noteID: note.id, store: store, send: { store.send(note) }, delete: { model.notes.removeAll { $0.id == note.id } },
                        moveBy: { delta in
                            if let from = ids.firstIndex(of: note.id), let to = Reorder.step(from, delta, count: ids.count) { store.moveNote(from: from, to: to) }
                        })
                        .reorderable(id: note.id, in: ids, dragging: $dragging, move: store.moveNote)
                        .walkRow("note-" + note.id.uuidString)
                        .transition(motion.row)
                }
            }
        }
        .animation(motion.list, value: model.notes.map(\.id))
        .animation(motion.fold, value: model.notesCollapsed)
    }
}
