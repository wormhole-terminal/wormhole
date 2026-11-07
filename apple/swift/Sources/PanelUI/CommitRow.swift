// One commit: the subject, who and when underneath, and the short hash on the right.

import DesignSystem
import PanelModel
import SwiftUI

struct CommitRow: View {
    let commit: Commit
    let now: Date
    let selected: Bool
    let space: Namespace.ID
    @State private var hovered = false
    @Environment(\.panelCopy) private var copy

    var body: some View {
        HStack(alignment: .top, spacing: Space._2) {
            VStack(alignment: .leading, spacing: 2) {
                Text(commit.subject).font(.callout).lineLimit(2)
                Text("\(commit.author) \u{00B7} \(RelativeDate.text(commit.date, now: now))")
                    .font(.subheadline).foregroundStyle(Palette.textSecondary).lineLimit(1)
            }
            Spacer(minLength: Space._1)
            Text(commit.shortHash)
                .font(.system(size: 11, design: .monospaced)).foregroundStyle(.tertiary)
        }
        .padding(.horizontal, Space._2)
        .padding(.vertical, Space._1 + 2)
        .contentShape(Rectangle())
        .selectionBackdrop(selected: selected, hovered: hovered, in: space)
        .onHover { hovered = $0 }
        .contextMenu {
            Button("Copy Hash") { copy(commit.hash) }
            Button("Copy Short Hash") { copy(commit.shortHash) }
            Button("Copy Subject") { copy(commit.subject) }
            Button("Copy Author") { copy(commit.author) }
        }
        .accessibilityElement(children: .ignore)
        .accessibilityLabel("\(commit.subject), by \(commit.author), \(RelativeDate.text(commit.date, now: now)), \(commit.shortHash)")
        .accessibilityAddTraits(selected ? [.isButton, .isSelected] : .isButton)
    }
}
