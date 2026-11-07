// The Notes card: its head (the notes with something in them, and a + that starts one) and the notes. A new
// note is empty and opens for writing at once.

import DesignSystem
import PanelModel
import SwiftUI

struct NotesCard: View {
    @Binding var model: NotesModel
    @Environment(\.panelMotion) private var motion

    var body: some View {
        PanelCard {
            CardBand(
                title: "Notes", folded: model.notes.isEmpty ? nil : model.notesCollapsed,
                count: model.filledNotes > 0 ? CountText.grouped(model.filledNotes) : nil,
                spoken: "Notes, \(model.filledNotes) written", onToggle: { model.notesCollapsed.toggle() },
                add: ("New note", { model.notesCollapsed = false; model.notes.append(Note(text: "")) }))
            if !model.notesCollapsed {
                ForEach($model.notes) { $note in
                    NoteRowView(
                        note: $note, send: {}, delete: { model.notes.removeAll { $0.id == note.id } })
                        .transition(motion.row)
                }
            }
        }
        .animation(motion.list, value: model.notes.map(\.id))
        .animation(motion.fold, value: model.notesCollapsed)
    }
}
