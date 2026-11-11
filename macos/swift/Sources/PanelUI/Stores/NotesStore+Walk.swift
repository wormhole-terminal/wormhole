// The Notes tab under the keys (`features/notes/list.rs`): the two bands (a ⇥ stop each), every next step, the "N done" fold and
// every note. A step being written is its editor's. Space ticks a step or folds a note, ↩ writes in it; a band or "N done"
// folds on Space (↩ does nothing on a band).

import Foundation
import PanelModel

extension NotesStore: PanelWalking {
    public var walkRows: [WalkRow] {
        var rows = [WalkRow(id: "band-steps", level: 0, isOpen: model.tasks.items.isEmpty ? nil : !model.stepsCollapsed, isBand: true, name: panelText("steps-title", "Next steps"))]
        if !model.stepsCollapsed {
            for row in model.tasks.rows(showDone: model.showDone) {
                switch row {
                case let .step(task): rows.append(WalkRow(id: "step-" + task.id.uuidString, level: 1, name: task.text))
                case .doneFold: rows.append(WalkRow(id: "done-fold", level: 1, isOpen: model.showDone, name: panelText("notes-task-done", "done")))
                }
            }
        }
        rows.append(WalkRow(id: "band-notes", level: 0, isOpen: model.notes.isEmpty ? nil : !model.notesCollapsed, isBand: true, name: panelText("notes-title", "Notes")))
        if !model.notesCollapsed { rows += model.notes.map { WalkRow(id: "note-" + $0.id.uuidString, level: 1, name: $0.title) } }
        return rows
    }

    public func press(_ row: WalkRow, space: Bool, following: Bool) {
        if let id = uuid(row, "step-") {
            if space { model.tasks.toggle(id) } else { freshStep = id }
        } else if let id = uuid(row, "note-"), let at = model.notes.firstIndex(where: { $0.id == id }) {
            if space { model.notes[at].collapsed.toggle() } else { (model.notes[at].collapsed, freshNote) = (false, id) }
        } else {
            fold(row)
        }
    }

    public func fold(_ row: WalkRow) {
        switch row.id {
        case "band-steps": model.stepsCollapsed.toggle()
        case "band-notes": model.notesCollapsed.toggle()
        case "done-fold": model.showDone.toggle()
        default: break
        }
    }

    private func uuid(_ row: WalkRow, _ prefix: String) -> UUID? {
        row.id.hasPrefix(prefix) ? UUID(uuidString: String(row.id.dropFirst(prefix.count))) : nil
    }
}
