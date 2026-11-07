// One result: an icon tile, the title with its matches marked, a dim subtitle and the keycaps.

import DesignSystem
import PaletteModel
import SwiftUI

struct PaletteRow: View {
    let result: PaletteResult
    let isSelected: Bool
    let isHovered: Bool
    let selectionNamespace: Namespace.ID
    let motion: PaletteMotion

    private var spokenLabel: String {
        [result.command.title, result.command.subtitle].compactMap { $0 }.joined(separator: ", ")
    }

    var body: some View {
        HStack(spacing: Space._3) {
            icon
            Text(MarkedText.title(result.command.title, ranges: result.ranges))
                .font(.system(size: 14))
                .lineLimit(1)
            if let subtitle = result.command.subtitle {
                Text(subtitle)
                    .font(.system(size: 12))
                    .foregroundStyle(Palette.textSecondary)
                    .lineLimit(1)
            }
            Spacer(minLength: Space._2)
            if let shortcut = result.command.shortcut { KeyCaps(shortcut: shortcut) }
        }
        .padding(.horizontal, Space._2)
        .frame(height: PaletteItem.rowHeight)
        .background { fill }
        .animation(motion.hover, value: isHovered)
        .contentShape(Rectangle())
        .accessibilityElement(children: .combine)
        .accessibilityLabel(Text(spokenLabel))
        .accessibilityAddTraits(isSelected ? [.isButton, .isSelected] : .isButton)
    }

    private var icon: some View {
        Image(systemName: result.command.symbol)
            .font(.system(size: 13))
            .foregroundStyle(isSelected ? Color.accentColor : Palette.textSecondary)
            .frame(width: Space._6 + Space._1, height: Space._6 + Space._1)
            .background(RoundedRectangle(cornerRadius: Radius.chip, style: .continuous).fill(Palette.hover))
            .accessibilityHidden(true)
    }

    /// The one highlight that slides: every row may draw it, only the selected one does.
    @ViewBuilder private var fill: some View {
        if isSelected {
            RoundedRectangle(cornerRadius: Radius.row, style: .continuous)
                .fill(Palette.selected)
                .matchedGeometryEffect(id: "selection", in: selectionNamespace)
        } else if isHovered {
            RoundedRectangle(cornerRadius: Radius.row, style: .continuous).fill(Palette.hover)
        }
    }
}
