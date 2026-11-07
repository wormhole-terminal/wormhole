// Undo and Redo in the title row, naming the step they would take back or do again (`Undo Change “primary”`) in
// the tooltip and for VoiceOver; disabled when there is nothing to do. ⌘Z and ⇧⌘Z are theirs while the Studio is
// in front, ahead of the text field, which keeps what was typed.

import DesignSystem
import StudioModel
import SwiftUI

struct UndoButtons: View {
    @Bindable var document: StudioDocument

    var body: some View {
        HStack(spacing: 0) {
            IconButton(symbol: "arrow.uturn.backward", label: LocalizedStringKey(StudioWords.undo(document.undoLabel))) { document.undo() }
                .disabled(document.undoLabel == nil).keyboardShortcut("z", modifiers: .command)
            IconButton(symbol: "arrow.uturn.forward", label: LocalizedStringKey(StudioWords.redo(document.redoLabel))) { document.redo() }
                .disabled(document.redoLabel == nil).keyboardShortcut("z", modifiers: [.command, .shift])
        }
    }
}
