// The small controls at a group head's end: the chevron in its 20 box that turns when the group folds, and the + that
// starts a terminal in the group (always shown on a pinned, empty group, else under the pointer).

import DesignSystem
import SwiftUI
import WordsModel

struct HeadChevron: View {
    let isFolded: Bool
    @State private var hovering = false
    @Environment(\.tokens) private var tokens

    var body: some View {
        GlyphView(glyph: .chevron, size: RowGeometry.chevron)
            .foregroundStyle(tokens.textMuted)
            .rotationEffect(.degrees(isFolded ? 0 : 90))
            .motionTurn(SidebarMotion.fold, value: isFolded)
            .frame(width: RowGeometry.chevronBox, height: RowGeometry.chevronBox)
            .background(hovering ? tokens.stateHover : .clear, in: RoundedRectangle(cornerRadius: tokens.metrics.radiusSm))
            .onHover { hovering = $0 }
            .motion(SidebarMotion.state, value: hovering)
            .accessibilityHidden(true)
    }
}

/// The + of a group (`groups.rs`: `icon_button(...).in_row()`): the shared icon button on a row's 22 pt face, `band.hover`
/// under the pointer, a quarter turn on a click.
struct HeadAddButton: View {
    /// The group's name: the tooltip and the label say which group the terminal opens in.
    let group: String
    let action: () -> Void

    private var label: String { Words.text("sidebar-new-in-group", ["group": .text(group)], fallback: "New terminal in \(group)") }

    var body: some View {
        IconButton(label: label, action: action) { _ in GlyphView(glyph: .plus, size: Metric.glyph) }
            .inRow()
            .answering(.quarter)
    }
}
