// A row of the folder picker: a folder's icon in the lead column, its name with the typed text marked, the branch
// badge of a repository, grey words after the label, and a chevron when there is more behind it. The selected row
// takes `list.selected`; one under the pointer takes `state.hover`.

import DesignSystem
import PlatformModel
import WordsModel
import SwiftUI
import WordsModel

struct FolderPickerRowView: View {
    let row: PickerRow
    let isSelected: Bool
    let isHovered: Bool
    @Environment(\.tokens) private var tokens

    var body: some View {
        HStack(spacing: Space._2) {
            Image(systemName: row.icon)
                .font(.system(size: Metric.glyph, weight: .light))
                .foregroundStyle(isSelected ? tokens.listSelectedMuted : tokens.textMuted)
                .frame(width: FolderPickerMetrics.icon)
            Text(marked)
                .font(.system(size: TextRole.body.size))
                .lineLimit(1)
                .layoutPriority(1)
            if let badge = row.badge { Badge(text: badge) }
            if let detail = row.detail {
                Text(detail)
                    .font(.system(size: TextRole.caption.size))
                    .foregroundStyle(isSelected ? tokens.listSelectedMuted : tokens.textMuted)
                    .lineLimit(1)
                    .truncationMode(.head)
            }
            Spacer(minLength: 0)
            if row.chevron {
                Image(systemName: "chevron.right")
                    .font(.system(size: Metric.chevron, weight: .semibold))
                    .foregroundStyle(isSelected ? tokens.listSelectedMuted : tokens.textFaint)
            }
        }
        .padding(.horizontal, Space._3)
        .frame(height: FolderPickerMetrics.row)
        .foregroundStyle(isSelected ? tokens.listSelectedText : tokens.textPrimary)
        .background {
            let shape = RoundedRectangle(cornerRadius: Radius.row)
            if isSelected { shape.fill(tokens.listSelected) } else if isHovered { shape.fill(tokens.stateHover) }
        }
        .contentShape(Rectangle())
        .accessibilityElement(children: .combine)
        .accessibilityLabel(Text(spoken))
        .accessibilityAddTraits(isSelected ? [.isButton, .isSelected] : .isButton)
    }

    /// What VoiceOver reads for the row: the name, the branch, the detail and whether it opens.
    private var spoken: String {
        let branch = row.badge.map { Words.text("title-row-branch-spoken", ["name": .text($0)], fallback: "branch \($0)") }
        let opens = row.chevron ? Words.text("platform-picker-has-folders", fallback: "has folders") : nil
        return [row.label, branch, row.detail, opens].compactMap { $0 }.joined(separator: ", ")
    }

    /// The label with the typed text marked in the primary colour and semibold, by character offsets.
    private var marked: AttributedString {
        var text = AttributedString(row.label)
        let characters = Array(text.characters.indices)
        for range in row.marks where range.lowerBound >= 0 && range.upperBound <= characters.count && !range.isEmpty {
            let end = range.upperBound == characters.count ? text.endIndex : characters[range.upperBound]
            text[characters[range.lowerBound]..<end].foregroundColor = isSelected ? tokens.listSelectedText : tokens.primary
            text[characters[range.lowerBound]..<end].font = .system(size: TextRole.body.size, weight: .semibold)
        }
        return text
    }

    /// The branch of a repository, `git.branch` on its own background, in the chip's shape.
    private struct Badge: View {
        let text: String
        @Environment(\.tokens) private var tokens

        var body: some View {
            Text(text)
                .font(.system(size: TextRole.caption.size, weight: .medium))
                .foregroundStyle(tokens.gitBranch)
                .padding(.horizontal, Space._1)
                .frame(height: Metric.badgeHeight)
                .background(tokens.gitBranchBackground, in: RoundedRectangle(cornerRadius: tokens.metrics.radiusChip))
                .lineLimit(1)
        }
    }
}
