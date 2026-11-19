// The overlay over the panes while a terminal is dragged: on the pane under the pointer, its other sides
// that would work as faint dashed hints with a chevron, and the side it is on as the share of the pane the
// terminal would take, in the drop tint with the primary's outline and a pill saying what a drop does
// (`style::pane_drop_zone`, `drop_hint`).

import DesignSystem
import PanesModel
import SwiftUI
import WorkspaceModel

struct PaneDropZones: View {
    let panes: PanesState
    let state: PaneDropState
    let size: CGSize
    let names: (PaneID) -> String
    let refusal: (DraggedTerminal, PaneID, Zone) -> Refusal?
    @Environment(\.tokens) private var tokens

    var body: some View {
        if let (dragged, target, zone) = state.landing,
            let pane = panes.shown.first(where: { $0.id == target })?.rect
        {
            let moving = panes.order.contains(dragged.id)
            ZStack(alignment: .topLeading) {
                // The sides that would work too, faint; the one under the pointer over them.
                ForEach(Zone.allCases.filter { $0 != zone }, id: \.self) { other in
                    if let band = DropBand.hint(pane, other, active: zone),
                        refusal(dragged, target, other) == nil
                    {
                        hint(other).placed(band)
                    }
                }
                if refusal(dragged, target, zone) == nil {
                    active(zone, text: PanesText.dropHint(zone, moving: moving, target: names(target))).placed(DropBand.preview(pane, zone))
                }
            }
            .frame(width: size.width, height: size.height, alignment: .topLeading)
            .allowsHitTesting(false)
            .transition(.opacity)
        }
    }

    private func hint(_ zone: Zone) -> some View {
        let shape = RoundedRectangle(cornerRadius: tokens.metrics.radiusSm)
        return shape.fill(tokens.stateHover)
            .overlay { shape.strokeBorder(tokens.borderStrong, style: StrokeStyle(lineWidth: tokens.metrics.borderWidth, dash: [4, 3])) }
            .overlay { Image(systemName: chevron(zone)).foregroundStyle(tokens.textMuted) }
    }

    private func active(_ zone: Zone, text: String) -> some View {
        let shape = RoundedRectangle(cornerRadius: tokens.metrics.radiusSm)
        return shape.fill(tokens.stateHighlight)
            .overlay { shape.strokeBorder(tokens.primary, lineWidth: PaneMetrics.dropLine) }
            .overlay {
                Text(text).font(tokens.font(.caption).weight(.semibold)).lineLimit(1).fixedSize()
                    .foregroundStyle(tokens.textOnPrimary)
                    .padding(.horizontal, Space._2).frame(height: Metric.controlSmall)
                    .background(tokens.primary, in: RoundedRectangle(cornerRadius: tokens.metrics.radiusChip))
            }
    }

    private func chevron(_ zone: Zone) -> String {
        switch zone {
        case .left: "chevron.left"
        case .right: "chevron.right"
        case .top: "chevron.up"
        case .bottom: "chevron.down"
        }
    }
}

extension View {
    /// Laid at `rect`, in the panes' area's own coordinates.
    func placed(_ rect: PaneRect) -> some View {
        frame(width: rect.w, height: rect.h).offset(x: rect.x, y: rect.y)
    }
}
