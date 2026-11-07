// The × that closes a terminal: it shows on the row under the pointer, the selected one too (`close_x`). Drawn 22
// square at the row's top right, its glyph ending where the first line does, with a hover fill of its own.

import DesignSystem
import SwiftUI

struct CloseButton: View {
    let isSelected: Bool
    let action: () -> Void
    @State private var hovering = false
    @Environment(\.tokens) private var tokens

    var body: some View {
        Button(action: action) {
            GlyphView(glyph: .close, size: Metric.glyph)
                .foregroundStyle(hovering ? (isSelected ? tokens.sidebarRowSelectedText : tokens.textPrimary)
                    : (isSelected ? tokens.sidebarRowSelectedText : tokens.textMuted))
                .frame(width: RowGeometry.closeBox, height: RowGeometry.closeBox)
                .background(hovering ? (isSelected ? tokens.stateSelectedInset : tokens.stateHover) : .clear,
                            in: RoundedRectangle(cornerRadius: tokens.metrics.radiusSm))
                .contentShape(Rectangle().inset(by: -(Metric.hitMin - RowGeometry.closeBox) / 2))
        }
        .buttonStyle(.plain)
        .onHover { hovering = $0 }
        .animation(SidebarMotion.state, value: hovering)
        .help("Close terminal")
        .accessibilityLabel("Close terminal")
    }
}
