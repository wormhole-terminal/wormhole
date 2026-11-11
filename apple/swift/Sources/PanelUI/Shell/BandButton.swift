// An icon button in a band or on a row: a 24 pt face with a 16 pt glyph in a band, a 22 pt face with a 14 pt
// glyph on a row, still clickable across the 28 pt hit minimum. Muted until the pointer is over it
// (`band.hover` fill, `text.primary`), `state.active` while pressed (`style::icon_button`).

import DesignSystem
import SwiftUI

struct BandButton: View {
    let symbol: String
    let label: String
    var inRow = false
    let action: () -> Void
    @State private var hovering = false
    @Environment(\.tokens) private var tokens

    var body: some View {
        let face = inRow ? PanelMetrics.rowButton : PanelMetrics.bandButton
        Button(action: action) {
            PanelIcon(symbol, size: inRow ? Metric.glyph : PanelMetrics.bandGlyph)
                .foregroundStyle(hovering ? tokens.textPrimary : tokens.textMuted)
                .frame(width: face, height: face)
                .background(hovering ? tokens.bandHover : .clear, in: RoundedRectangle(cornerRadius: tokens.metrics.radiusSm))
                .frame(width: Metric.hitMin, height: Metric.hitMin)
                .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        // The hit box is wider than the face; the pull keeps the glyph, not the box, on the row's inset.
        .padding(.horizontal, -(Metric.hitMin - face) / 2)
        .onHover { hovering = $0 }
        .help(label)
        .accessibilityLabel(label)
    }
}
