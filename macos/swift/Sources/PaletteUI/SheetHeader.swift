// The sheet's header: the keyboard icon, the title, the filter in a field of its own and the esc key cap, a
// hairline under it.

import DesignSystem
import SwiftUI
import WordsModel

struct SheetHeader: View {
    @Binding var filter: String
    let onKey: (FieldKey) -> Bool
    @Environment(\.tokens) private var tokens

    var body: some View {
        HStack(spacing: Space._3) {
            Image(systemName: "keyboard")
                .font(.system(size: Metric.glyph))
                .foregroundStyle(tokens.textMuted)
                .accessibilityHidden(true)
            Text(Words.text("palette-sheet-title", fallback: "Keyboard shortcuts"))
                .font(.system(size: tokens.size(.heading), weight: .bold))
                .accessibilityAddTraits(.isHeader)
            Spacer(minLength: 0)
            filterField
            KeyCap(text: "esc")
        }
        .padding(.leading, OverlayMetrics.sheetMargin)
        .padding(.trailing, Space._4)
        .frame(height: OverlayMetrics.sheetHeaderHeight)
        .overlay(alignment: .bottom) { Rule(color: tokens.border, width: tokens.metrics.borderWidth) }
    }

    private var filterField: some View {
        let shape = RoundedRectangle(cornerRadius: tokens.metrics.radius)
        return HStack(spacing: Space._2) {
            Image(systemName: "magnifyingglass")
                .font(.system(size: Metric.glyph))
                .foregroundStyle(tokens.textMuted)
                .accessibilityHidden(true)
            PaletteField(
                text: $filter, placeholder: Words.text("palette-sheet-filter", fallback: "Filter shortcuts"), size: tokens.size(.label), textColor: tokens.textPrimary,
                placeholderColor: tokens.inputPlaceholder, caretColor: tokens.textPrimary, onKey: onKey)
                .padding(.leading, Space._2)
        }
        .padding(.horizontal, Space._2)
        .frame(width: OverlayMetrics.sheetFilterWidth, height: Metric.control)
        .background(tokens.inputBackground, in: shape)
        .overlay { shape.strokeBorder(tokens.borderStrong, lineWidth: tokens.metrics.borderWidth) }
    }
}
