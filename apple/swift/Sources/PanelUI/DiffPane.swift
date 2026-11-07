// The diff of one file: its name, then hunks. Binary and empty diffs say so instead of showing nothing.

import DesignSystem
import PanelModel
import SwiftUI

struct DiffPane: View {
    let file: ChangedFile
    let diff: DiffFile?

    var body: some View {
        VStack(spacing: 0) {
            header
            Rectangle().fill(Palette.divider).frame(height: 1)
            if let diff, diff.isBinary {
                EmptyState(symbol: "photo", title: "Binary file", detail: "It changed, but there is no text to show.")
            } else if let diff, !diff.hunks.isEmpty {
                ScrollView {
                    LazyVStack(spacing: Space._1) {
                        ForEach(Array(diff.hunks.enumerated()), id: \.offset) { _, hunk in DiffHunkView(hunk: hunk) }
                    }
                    .padding(.bottom, Space._2)
                }
            } else {
                EmptyState(
                    symbol: "doc.text", title: "No diff to show",
                    detail: file.status == .untracked ? "New files are not tracked until they are added." : "Nothing changed in the text.")
            }
        }
    }

    private var header: some View {
        HStack(spacing: Space._2) {
            Text(headline).font(.callout.weight(.medium)).lineLimit(1).truncationMode(.middle)
            Spacer(minLength: 0)
            ChangeCounts(added: file.added, removed: file.removed)
        }
        .padding(.horizontal, Space._3)
        .frame(height: 32)
        .accessibilityElement(children: .combine)
        .accessibilityAddTraits(.isHeader)
    }

    /// A rename reads as a move: `old → new`, since the new path alone hides what the file used to be called.
    private var headline: String {
        guard let diff, diff.isRenamed, let old = diff.oldPath else { return file.name }
        return "\(old.split(separator: "/").last ?? "") \u{2192} \(file.name)"
    }
}
