// What the diff says where it shows no lines (`features/git/diff/notes.rs`): a small chip on the card's surface so
// it reads over the hatch.

import DesignSystem
import PanelModel
import SwiftUI

struct DiffNote: View {
    let text: String
    @Environment(\.tokens) private var tokens

    var body: some View {
        Text(text).font(TextRole.caption.font).foregroundStyle(tokens.textMuted)
            .padding(.horizontal, Space._2).padding(.vertical, Space._1)
            .background(tokens.surfaceWindow, in: RoundedRectangle(cornerRadius: tokens.metrics.radiusChip))
            .overlay { RoundedRectangle(cornerRadius: tokens.metrics.radiusChip).strokeBorder(tokens.border, lineWidth: tokens.metrics.borderWidth) }
    }

    /// The note for a file with no rows: why, from what the diff and the change say.
    static func text(for file: ChangedFile, diff: DiffFile?) -> String? {
        if diff?.isBinary == true { return "Binary file" }
        if file.lfs { return "Stored in Git LFS" }
        guard diff?.hunks.isEmpty ?? true else { return nil }
        return file.kind == .added || file.kind == .untracked ? "Empty file" : "No content changes"
    }
}
