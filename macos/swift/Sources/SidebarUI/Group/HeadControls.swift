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
            .motion(SidebarMotion.fold, value: isFolded)
            .frame(width: RowGeometry.chevronBox, height: RowGeometry.chevronBox)
            .background(hovering ? tokens.stateHover : .clear, in: RoundedRectangle(cornerRadius: tokens.metrics.radiusSm))
            .onHover { hovering = $0 }
            .motion(SidebarMotion.state, value: hovering)
            .accessibilityHidden(true)
    }
}

struct HeadAddButton: View {
    /// The group's name: the tooltip and the label say which group the terminal opens in.
    let group: String
    let action: () -> Void
    @State private var hovering = false
    private let run = IconRun()
    @Environment(\.tokens) private var tokens

    private var label: String { Words.text("sidebar-new-in-group", ["group": .text(group)], fallback: "New terminal in \(group)") }

    var body: some View {
        Button {
            run.start()
            action()
        } label: {
            GlyphView(glyph: .plus, size: Metric.titleGlyph)
                .iconMotion(.quarter, run: run.count)
                .foregroundStyle(hovering ? tokens.textPrimary : tokens.textMuted)
                .frame(width: Metric.titleControl, height: Metric.titleControl)
                .background(hovering ? tokens.stateHover : .clear, in: RoundedRectangle(cornerRadius: tokens.metrics.radiusSm))
        }
        .buttonStyle(.plain)
        .onHover { hovering = $0 }
        .help(label)
        .accessibilityLabel(label)
    }
}
