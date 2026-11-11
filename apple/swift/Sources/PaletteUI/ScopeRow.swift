// The scope row: All and the kinds, each with its prefix, in the app's one segmented control: a track in
// `state.hover` round the segments, the chosen one solid (`chip.selected`, `text.on_selected`).

import DesignSystem
import PaletteModel
import SwiftUI
import WordsModel

struct ScopeRow: View {
    let scope: PaletteScope
    let choose: (PaletteScope) -> Void
    @State private var hovered: PaletteScope?
    @Environment(\.tokens) private var tokens

    var body: some View {
        HStack(spacing: 0) {
            ForEach(PaletteScope.row, id: \.self) { item in segment(item) }
        }
        .padding(OverlayMetrics.segmentInset)
        .background(
            tokens.stateHover,
            in: RoundedRectangle(cornerRadius: tokens.metrics.radiusChip + OverlayMetrics.segmentInset)
        )
        .padding(.horizontal, Space._3)
        .frame(maxWidth: .infinity, alignment: .leading)
        .frame(height: Metric.paletteRowHeight)
        .overlay(alignment: .bottom) { Rule(color: tokens.border, width: tokens.metrics.borderWidth) }
        .accessibilityElement(children: .contain)
        .accessibilityLabel(Text(Words.text("palette-scope-label", fallback: "Scope")))
    }

    private func segment(_ item: PaletteScope) -> some View {
        let active = item == scope
        let label = item.prefix.map { "\($0) \(item.title)" } ?? item.title
        return Button { choose(item) } label: {
            Text(label)
                .font(.system(size: TextRole.caption.size))
                .foregroundStyle(active ? tokens.textOnSelected : hovered == item ? tokens.textPrimary : tokens.textMuted)
                .padding(.horizontal, Space._2)
                .frame(height: Metric.controlSmall - 2 * OverlayMetrics.segmentInset)
                .background {
                    if active {
                        RoundedRectangle(cornerRadius: tokens.metrics.radiusChip).fill(tokens.chipSelected)
                    }
                }
                .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .onHover { inside in hovered = inside ? item : (hovered == item ? nil : hovered) }
        .accessibilityAddTraits(active ? .isSelected : [])
    }
}

/// A hairline along one edge of a row (`rule_top`, `rule_bottom`); it keeps its space whatever its colour.
struct Rule: View {
    let color: Color
    let width: CGFloat

    var body: some View { color.frame(height: width).allowsHitTesting(false) }
}
