// The divider between two panes: a handle straddling their shared edge (it lies in the gap itself when the
// panes are their own surfaces) with the hairline along the edge, which takes `resize.handle.hover` under the
// pointer after a short rest and `resize.handle.active` while dragged, with a grabber pill across it. A
// row split's divider is vertical and drags sideways, a column split's horizontal. Double-click equalizes
// (`style::pane_divider`).

import DesignSystem
import PanesModel
import SwiftUI
import WorkspaceModel

enum DividerLook { case rest, hot, dragging }

struct PaneDividerView: View {
    let axis: SplitAxis
    let gaps: Bool
    let look: DividerLook
    @Environment(\.tokens) private var tokens

    private var line: Color {
        switch look {
        case .rest: tokens.border
        case .hot: tokens.resizeHandleHover
        case .dragging: tokens.resizeHandleActive
        }
    }

    /// How thick the handle is across the edge: the gap itself, or the straddling handle.
    var thickness: CGFloat { gaps ? PaneMetrics.gap : PaneMetrics.handle }

    var body: some View {
        let vertical = axis == .row
        ZStack {
            // With gaps no hairline is drawn, only the grabber.
            Rectangle().fill(gaps ? .clear : line)
                .frame(width: vertical ? tokens.metrics.borderWidth : nil, height: vertical ? nil : tokens.metrics.borderWidth)
            if look != .rest {
                RoundedRectangle(cornerRadius: tokens.metrics.radiusChip)
                    .fill(line)
                    .frame(width: vertical ? thickness : PaneMetrics.grabber, height: vertical ? PaneMetrics.grabber : thickness)
                    .transition(.opacity)
            }
        }
        .frame(width: vertical ? thickness : nil, height: vertical ? nil : thickness)
        .marked("column.divider")
        .contentShape(Rectangle())
        .pointerStyle(vertical ? .columnResize : .rowResize)
        .motion(Motion.press, value: look)
        .help(PanesText.dividerHint)
        .accessibilityElement()
        .accessibilityLabel(PanesText.divider)
        .accessibilityAdjustableAction { _ in }
    }
}
