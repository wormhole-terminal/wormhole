// A tab: a box of its own on the terminal's colour, the open one filled most, with no edge on either so
// switching tabs moves nothing. Its tile (the row's own, with the agent's logo and the status mark, when the window gives one), its name (one weight for every tab: a bolder one on the open tab
// would change how much of it fits), the ⌘N keycap while ⌘ is held, and a × on hover. A long name fades out
// over its end rather than ending in an ellipsis.

import DesignSystem
import PanesModel
import SwiftUI

struct TerminalTabView: View {
    let tab: TabItem
    let selected: Bool
    let hints: Bool
    var tile: AnyView?
    let close: () -> Void
    @State private var hovering = false
    @Environment(\.tokens) private var tokens
    @Environment(\.cardGlass) private var glass

    var body: some View {
        let shape = RoundedRectangle(cornerRadius: tokens.metrics.radius)
        HStack(spacing: Space._2) {
            if let tile { tile } else { TerminalTile(activity: tab.activity, size: TabMetrics.tile) }
            // The name takes the room the tab has (the keycap's included) and is cut by the fade, never pushing the keycap out.
            Color.clear.frame(maxWidth: .infinity, maxHeight: .infinity)
                .overlay(alignment: .leading) {
                    Text(tab.name).font(TextRole.label.font.weight(.medium))
                        .foregroundStyle(selected ? tokens.textPrimary : tokens.textMuted)
                        .lineLimit(1).fixedSize()
                }
                .mask { fade }
            if hints, let number = tab.number { keycap(number) }
        }
        .padding(.horizontal, Space._2)
        .frame(width: TabMetrics.width, height: TabMetrics.height)
        .background { if glass { shape.fill(.clear).glassEffect(.regular, in: shape) } }
        .background(shape.fill(selected ? tokens.textPrimary.opacity(TabMetrics.openFill) : hovering ? tokens.stateHover : tokens.textPrimary.opacity(TabMetrics.closedFill)))
        .overlay(alignment: .trailing) { if hovering && !hints { closeButton } }
        .clipShape(shape)
        .marked("title.tab")
        .contentShape(shape)
        .onHover { hovering = $0 }
        .motion(Motion.selection, value: selected)
        .accessibilityElement(children: .combine)
        // The core's words (name, state, selection, "n of m") where it gave them.
        .accessibilityLabel(tab.spoken ?? PanesText.tabLabel(tab.name, selected: selected))
        .accessibilityAddTraits(selected ? [.isButton, .isSelected] : .isButton)
    }

    /// The end of a name that does not fit fades out instead of ending in an ellipsis.
    private var fade: some View {
        HStack(spacing: 0) {
            Rectangle()
            LinearGradient(colors: [.black, .clear], startPoint: .leading, endPoint: .trailing).frame(width: Space._4)
        }
    }

    private func keycap(_ number: Int) -> some View {
        Text("⌘\(number)").font(TextRole.caption.font.weight(.medium)).foregroundStyle(tokens.textMuted)
            .padding(.horizontal, Space._1).frame(minWidth: Metric.keyCapMinWidth - 8, minHeight: Metric.keyCapHeight)
            .background(tokens.border, in: RoundedRectangle(cornerRadius: tokens.metrics.radiusChip))
    }

    private var closeButton: some View {
        PaneButton(symbol: "xmark", label: PanesText.closeTerminal, action: close)
            .padding(.trailing, Space._1)
            .transition(.opacity)
    }
}
