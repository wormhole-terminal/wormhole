// The size chip, drawn from `SizeReadoutModel`: a tooltip-coloured pill (body size, chip corners, 12 / 4 padding) in
// the middle of the terminal. A timeline walks the phases, so there is no timer to cancel; it pauses once the chip is
// gone and a new size starts a new body. It takes no clicks: the terminal below it still gets them.

import DesignSystem
import SwiftUI

public struct SizeReadoutChip: View {
    let model: SizeReadoutModel

    public init(model: SizeReadoutModel) { self.model = model }

    public var body: some View {
        if let text = model.text {
            ChipBody(text: text, since: model.since)
                .id(model.since)
                .allowsHitTesting(false)
                .frame(maxWidth: .infinity, maxHeight: .infinity)
        }
    }
}

private struct ChipBody: View {
    let text: String
    let since: Date
    @State private var finished = false
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @Environment(\.tokens) private var tokens

    var body: some View {
        TimelineView(.animation(paused: finished)) { timeline in
            let phase = SizeReadoutPhase.of(since: timeline.date.timeIntervalSince(since), cut: reduceMotion)
            ReadoutPill(role: .body, cornerRadius: tokens.metrics.radiusChip) {
                Text(text).padding(.horizontal, Space._3).padding(.vertical, Space._1)
            }
            .opacity(opacity(phase))
            .onChange(of: phase == .gone) { _, gone in finished = gone }
            .accessibilityLabel(text)
        }
    }

    private func opacity(_ phase: SizeReadoutPhase) -> Double {
        switch phase {
        case .holding: 1
        case let .fading(left): left
        case .gone: 0
        }
    }
}
