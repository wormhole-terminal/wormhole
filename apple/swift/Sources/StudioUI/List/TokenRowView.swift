// One token in the list: its swatch in the lead column, its name in the code font, a warning when a pair it is
// part of falls short, a dot when it was edited, the value faint on the right, and for a pair of versions the
// chip that says which of them the value is for (`token_row` of the GPUI app's list).

import DesignSystem
import StudioModel
import SwiftUI

struct TokenRowView: View {
    let name: String
    let value: TokenRowValue
    let isSelected: Bool
    let isLow: Bool
    let isChanged: Bool
    let origin: TokenOrigin
    let selection: Namespace.ID
    let select: () -> Void
    @Environment(\.tokens) private var tokens

    var body: some View {
        Row(isSelected: isSelected, selection: selection, action: select) { _ in
            HStack(spacing: Space._2) {
                TokenSwatch(chip: value.chip).frame(width: Metric.lead + Space._1)
                Text(name).font(TextRole.caption.font.monospaced()).lineLimit(1).truncationMode(.middle)
                if isLow { Image(systemName: "exclamationmark.triangle.fill").foregroundStyle(tokens.statusWarning).font(TextRole.caption.font) }
                if isChanged { ChangedDot() }
                Spacer(minLength: Space._2)
                Text(origin == .yours ? StudioWords.originYours : StudioWords.originWorkedOut)
                    .font(TextRole.caption.font).foregroundStyle(muted)
                    .help(origin == .yours ? StudioWords.tipYours : StudioWords.tipWorkedOut)
                Text(value.text).font(TextRole.caption.font.monospaced()).foregroundStyle(muted).lineLimit(1)
                    .truncationMode(.tail).frame(maxWidth: StudioMetrics.valueMaxWidth, alignment: .trailing)
            }
            .frame(height: Metric.row)
            .foregroundStyle(isSelected ? tokens.sidebarRowSelectedText : tokens.textPrimary)
        }
        .padding(.horizontal, Space._2)
        .accessibilityElement(children: .ignore)
        .accessibilityLabel("\(name), \(value.text), \(origin == .yours ? StudioWords.originYours : StudioWords.originWorkedOut)")
        .accessibilityHint(isLow ? StudioWords.rowLow : "")
        .accessibilityAddTraits(isSelected ? [.isButton, .isSelected] : .isButton)
    }

    private var muted: Color { isSelected ? tokens.sidebarRowSelectedMuted : tokens.textFaint }
}
