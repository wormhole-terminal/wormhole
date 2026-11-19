// A file's added and removed lines, "+N −M" in the diff colours, caption digits that keep their width
// (`style::line_counts`). A file that added nothing says only what it removed, and vice versa for zero removals.

import DesignSystem
import PanelModel
import SwiftUI

struct LineCounts: View {
    let added: Int
    let removed: Int
    @Environment(\.tokens) private var tokens

    var body: some View {
        HStack(spacing: Space._1) {
            Text(verbatim: "+" + String(added)).foregroundStyle(tokens.diffAdded)
            if removed > 0 { Text(verbatim: "\u{2212}" + String(removed)).foregroundStyle(tokens.diffRemoved) }
        }
        .font(tokens.font(.caption).monospacedDigit())
        .fixedSize()
    }
}

/// What a file row says of its lines, right before the kind letter: counts, "bin", "LFS" or a submodule's words.
struct FileLinesLabel: View {
    @Environment(\.tokens) private var tokens
    let lines: ChangedFile.Lines
    let muted: Color

    var body: some View {
        switch lines {
        case let .counts(added, removed): LineCounts(added: added, removed: removed)
        case .binary: word(panelText("changes-badge-binary", "bin"))
        case .lfs: word("LFS") // text: Git LFS, a product name
        case let .submodule(words): word(words)
        case .folder: EmptyView()
        }
    }

    private func word(_ text: String) -> some View {
        Text(text).font(tokens.font(.caption)).foregroundStyle(muted).fixedSize()
    }
}
