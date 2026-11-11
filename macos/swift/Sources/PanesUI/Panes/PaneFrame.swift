// One pane: its header and its terminal, the terminal dimmed (72 %) while another pane has the keyboard.
// With gaps each pane is its own rounded surface with an outline that says which has the keyboard: a quiet
// grey on the focused pane, a step up from the hairline on the others, since they are dimmed too and a
// coloured ring shouted (`style::pane_outline`).

import DesignSystem
import SwiftUI

struct PaneFrame<Header: View, Content: View>: View {
    let focused: Bool
    let gaps: Bool
    let lifted: Bool
    @ViewBuilder let header: () -> Header
    @ViewBuilder let content: () -> Content
    @Environment(\.tokens) private var tokens

    var body: some View {
        let shape = RoundedRectangle(cornerRadius: gaps ? tokens.metrics.radiusLg : 0, style: .continuous)
        VStack(spacing: 0) {
            header()
            content()
                // A pane that is its own box keeps its text off the box's edge, under the header's tile.
                .padding(.horizontal, gaps ? PaneMetrics.textPad : 0)
                .padding(.bottom, gaps ? PaneMetrics.textPad : 0)
                .opacity(focused ? 1 : PaneMetrics.dim)
                .frame(maxWidth: .infinity, maxHeight: .infinity)
        }
        .background { if gaps { shape.fill(tokens.surfaceWindow) } }
        .clipShape(shape)
        .overlay {
            if gaps {
                shape.strokeBorder(focused ? tokens.textFaint : tokens.border, lineWidth: tokens.metrics.borderWidth)
                    .allowsHitTesting(false)
            }
        }
        .opacity(lifted ? PaneMetrics.dragLift : 1)
        .motion(Motion.selection, value: focused)
    }
}
