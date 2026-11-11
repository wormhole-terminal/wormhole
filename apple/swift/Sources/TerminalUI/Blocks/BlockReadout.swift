// The readout of the hovered block (`style::block_readout`): its exit code (green for 0, red otherwise), its time
// and Copy output, on a tooltip-coloured pill at the pane's bottom right. It stays while the pointer is on it.

import DesignSystem
import SwiftUI

public struct BlockReadout: View {
    let controller: BlockController
    let state: BlockState
    let id: UInt64
    @Environment(\.tokens) private var tokens

    public init(controller: BlockController, id: UInt64, state: BlockState) {
        (self.controller, self.id, self.state) = (controller, id, state)
    }

    public var body: some View {
        if case let .finished(code, took) = state {
            ReadoutPill {
                HStack(spacing: Space._1) {
                    if let code {
                        Text(L10n.blockExit(code)).foregroundStyle(code == 0 ? tokens.tooltipSuccess : tokens.tooltipDanger)
                    }
                    if let took { Text(code == nil ? BlockWording.took(took) : "· " + BlockWording.took(took)) }
                    IconButton(lucide: .copy, label: LocalizedStringKey(L10n.blockCopyOutput)) {
                        controller.perform(.copyOutput, on: controller.block(id: id))
                    }
                    .foregroundStyle(tokens.tooltipText)
                }
                .padding(.leading, Space._2).padding(.trailing, Space._1)
                .frame(height: Metric.control)
            }
            .onHover { controller.readoutHovered = $0 }
            .accessibilityElement(children: .contain)
            .accessibilityLabel(BlockWording.readout(state) ?? "")
            .transition(.opacity)
        }
    }
}
