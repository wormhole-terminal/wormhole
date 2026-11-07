// The small controls at a group head's end: the chevron in its 20 box that turns when the group folds, and the + that
// starts a terminal in the group (always shown on a pinned, empty group, else under the pointer).

import DesignSystem
import SwiftUI

struct HeadChevron: View {
    let isFolded: Bool
    @State private var hovering = false
    @Environment(\.tokens) private var tokens

    var body: some View {
        GlyphView(glyph: .chevron, size: RowGeometry.chevron)
            .foregroundStyle(tokens.textMuted)
            .rotationEffect(.degrees(isFolded ? 0 : 90))
            .animation(SidebarMotion.fold, value: isFolded)
            .frame(width: RowGeometry.chevronBox, height: RowGeometry.chevronBox)
            .background(hovering ? tokens.stateHover : .clear, in: RoundedRectangle(cornerRadius: tokens.metrics.radiusSm))
            .onHover { hovering = $0 }
            .animation(SidebarMotion.state, value: hovering)
            .accessibilityHidden(true)
    }
}

struct HeadAddButton: View {
    let group: String
    let action: () -> Void
    @State private var hovering = false
    @Environment(\.tokens) private var tokens

    var body: some View {
        Button(action: action) {
            GlyphView(glyph: .plus, size: Metric.titleGlyph)
                .foregroundStyle(hovering ? tokens.textPrimary : tokens.textMuted)
                .frame(width: Metric.titleControl, height: Metric.titleControl)
                .background(hovering ? tokens.stateHover : .clear, in: RoundedRectangle(cornerRadius: tokens.metrics.radiusSm))
        }
        .buttonStyle(.plain)
        .onHover { hovering = $0 }
        .help("New terminal in group")
        .accessibilityLabel("New terminal in group")
    }
}
