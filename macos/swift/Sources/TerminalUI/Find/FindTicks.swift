// Find's ticks along the pane's right edge, in its padding beside the grid (`overlays::paint_ticks`): one per
// match, one per pixel row however many matches share it, the grid's height standing for the whole scrollback.

import DesignSystem
import SwiftUI

public struct FindTicks: View {
    let positions: [Double]
    @Environment(\.tokens) private var tokens

    public init(positions: [Double]) { self.positions = positions }

    public var body: some View {
        Canvas { context, size in
            let travel = size.height - FindMetrics.tickHeight
            var last: CGFloat?
            for at in positions {
                let y = (CGFloat(at) * travel).rounded()
                if y == last { continue }
                last = y
                let tick = CGRect(x: size.width - FindMetrics.tickInset - FindMetrics.tickWidth, y: y,
                                  width: FindMetrics.tickWidth, height: FindMetrics.tickHeight)
                context.fill(Path(roundedRect: tick, cornerRadius: FindMetrics.tickHeight / 2), with: .color(tokens.findMatchCurrent))
            }
        }
        .allowsHitTesting(false)
        .accessibilityHidden(true)
    }
}
