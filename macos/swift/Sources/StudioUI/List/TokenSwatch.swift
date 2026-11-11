// What a token row (and the editor) draws for its value: a rounded swatch of the colour, over a checker so a
// translucent one reads as translucent, or for a metric a corner or an outline of that many points
// (`color_chip` and `metric_chip` of the GPUI app's Studio).

import DesignSystem
import StudioModel
import SwiftUI

struct TokenSwatch: View {
    let chip: TokenRowValue.Chip
    var size = StudioMetrics.swatch
    @Environment(\.tokens) private var tokens

    var body: some View {
        switch chip {
        case .color(let color):
            let shape = RoundedRectangle(cornerRadius: tokens.metrics.radiusSm)
            shape.fill(checker).overlay(shape.fill(color.color))
                .overlay(shape.strokeBorder(tokens.borderStrong, lineWidth: tokens.metrics.borderWidth))
                .frame(width: size, height: size)
        case .metric(let pixels):
            // A corner as it would be drawn, capped at half the swatch (fully round).
            let radius = min(pixels, size / 2)
            RoundedRectangle(cornerRadius: radius)
                .strokeBorder(tokens.textMuted, lineWidth: max(1, min(pixels, 3)))
                .frame(width: size, height: size)
        }
    }

    /// Two greys of the theme's own text colour, at the strength a hairline is.
    private var checker: some ShapeStyle { tokens.textFaint.opacity(0.35) }
}
