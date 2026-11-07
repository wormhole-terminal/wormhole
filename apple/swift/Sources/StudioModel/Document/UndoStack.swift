// Every change to the file, to undo and redo; kept across a save. A step is the file as it stood after the
// change and what the change was, which is what the Edit menu and the title row's buttons name.

struct UndoStack: Equatable {
    struct Step: Equatable {
        let label: String
        let file: ThemeFile
    }

    private var steps: [Step]
    private var cursor = 0

    init(start: ThemeFile) { steps = [Step(label: "", file: start)] }

    var canUndo: Bool { cursor > 0 }
    var canRedo: Bool { cursor < steps.count - 1 }
    /// What undo would take back, and what redo would do again.
    var undoLabel: String? { canUndo ? steps[cursor].label : nil }
    var redoLabel: String? { canRedo ? steps[cursor + 1].label : nil }

    mutating func record(_ label: String, _ file: ThemeFile) {
        guard file != steps[cursor].file else { return }
        steps.removeSubrange((cursor + 1)...)
        steps.append(Step(label: label, file: file))
        cursor += 1
    }

    mutating func undo() -> ThemeFile? {
        guard canUndo else { return nil }
        cursor -= 1
        return steps[cursor].file
    }

    mutating func redo() -> ThemeFile? {
        guard canRedo else { return nil }
        cursor += 1
        return steps[cursor].file
    }
}
