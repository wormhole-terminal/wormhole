// The field that stands in a row's place while it is renamed (`rename.rs`): the name selected but not its extension, as
// Finder does. ↩ commits; esc or a click elsewhere cancels. Refused, it stays open with what was typed and says why.

import DesignSystem
import PanelModel
import SwiftUI

struct RenameField: View {
    let store: FilesStore
    let row: FileRow
    @State private var draft = ""
    @State private var selection: TextSelection?
    @FocusState private var focused: Bool
    @Environment(\.tokens) private var tokens

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            TextField("", text: $draft, selection: $selection)
                .textFieldStyle(.plain).font(tokens.font(.body)).foregroundStyle(tokens.textPrimary).focused($focused)
                .onSubmit { store.commitRename(to: draft) }
                .onExitCommand(perform: store.cancelRename)
                .onChange(of: focused) { _, now in if !now { store.cancelRename() } }
                .padding(.horizontal, Space._1).frame(height: Metric.controlSmall)
                .background(tokens.inputBackground, in: RoundedRectangle(cornerRadius: tokens.metrics.radiusSm))
                .overlay { RoundedRectangle(cornerRadius: tokens.metrics.radiusSm).strokeBorder(tokens.inputBorderFocus, lineWidth: tokens.metrics.borderWidth) }
                .accessibilityLabel(panelText("files-spoken-rename-field", "New name, text field"))
            if let error = store.renameError { Text(error).font(tokens.font(.caption)).foregroundStyle(tokens.statusDangerText) }
        }
        .onAppear {
            draft = row.name
            let stem = FileNames.stemLength(row.name, isFolder: row.isFolder)
            selection = TextSelection(range: draft.startIndex..<draft.index(draft.startIndex, offsetBy: stem))
            focused = true
        }
    }
}
