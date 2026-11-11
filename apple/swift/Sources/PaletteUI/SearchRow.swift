// The search row along a palette's top (`style::palette_search`): the magnifier at x 16, the scope as a tinted chip
// once one is chosen, the field at 16 pt, the esc key cap at the right, a hairline under it.

import DesignSystem
import PaletteModel
import SwiftUI

struct SearchRow: View {
    let scope: PaletteScope
    @Binding var text: String
    let onKey: (FieldKey) -> Bool
    @Environment(\.tokens) private var tokens

    var body: some View {
        HStack(spacing: Space._2) {
            Image(systemName: "magnifyingglass")
                .font(.system(size: Metric.titleGlyph, weight: .light))
                .foregroundStyle(tokens.textMuted)
                .accessibilityHidden(true)
            if scope != .all { chip }
            PaletteField(
                text: $text, placeholder: scope.placeholder, size: TextRole.heading.size, textColor: tokens.textPrimary,
                placeholderColor: tokens.inputPlaceholder, caretColor: tokens.textPrimary, onKey: onKey)
                .padding(.leading, Space._2)
            KeyCap(text: "esc")
        }
        .padding(.horizontal, Space._4)
        .frame(height: Metric.paletteSearchHeight)
        .overlay(alignment: .bottom) { Rule(color: tokens.border, width: tokens.metrics.borderWidth) }
    }

    /// `style::tint_chip`: the primary's tint with its text, the kind the palette is narrowed to.
    private var chip: some View {
        Text(scope.title)
            .font(.system(size: TextRole.caption.size, weight: .medium))
            .foregroundStyle(tokens.sidebarRowSelectedText)
            .padding(.horizontal, Space._2)
            .frame(height: OverlayMetrics.chipHeight)
            .background(tokens.chipTint, in: RoundedRectangle(cornerRadius: tokens.metrics.radiusChip))
            .fixedSize()
            .transition(.opacity)
    }
}
