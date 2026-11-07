// A hunk: a header that folds it, then its lines. The chevron turns and the lines slide away in one
// transaction; the header stays so the file's shape is still readable when everything is folded.

import DesignSystem
import PanelModel
import SwiftUI

struct DiffHunkView: View {
    let hunk: DiffHunk
    @State private var folded = false
    @Environment(\.panelMotion) private var motion
    @Environment(\.panelCopy) private var copy

    var body: some View {
        VStack(spacing: 0) {
            Button {
                withAnimation(motion.fold) { folded.toggle() }
            } label: {
                HStack(spacing: Space._2) {
                    Image(systemName: "chevron.right")
                        .font(.system(size: 9, weight: .bold))
                        .rotationEffect(.degrees(folded || motion.reduced ? 0 : 90))
                        .foregroundStyle(.tertiary)
                        .frame(width: 10)
                    Text(hunk.range).font(.system(size: 11, design: .monospaced)).foregroundStyle(.secondary)
                    if !hunk.section.isEmpty {
                        Text(hunk.section).font(.subheadline).foregroundStyle(.tertiary).lineLimit(1)
                    }
                    Spacer(minLength: 0)
                    ChangeCounts(added: hunk.added, removed: hunk.removed)
                }
                .padding(.horizontal, Space._2)
                .frame(height: 24)
                .background(PanelColors.hunkHeader)
                .contentShape(Rectangle())
            }
            .buttonStyle(.plain)
            .contextMenu {
                Button("Copy Hunk") { copy(hunk.lines.map(\.text).joined(separator: "\n")) }
            }
            .accessibilityLabel("Changes at \(hunk.range) \(hunk.section)")
            .accessibilityValue(folded ? "Folded" : "Expanded")
            .accessibilityHint("Folds or unfolds the hunk")

            if !folded {
                VStack(spacing: 0) {
                    ForEach(Array(hunk.lines.enumerated()), id: \.offset) { _, line in DiffLineRow(line: line) }
                }
                .transition(motion.hunk)
            }
        }
        .clipped()
    }
}
