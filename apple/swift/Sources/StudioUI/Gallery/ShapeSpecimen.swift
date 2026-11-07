// What a theme's metrics shape: the four corner roles side by side, a hairline and a column divider at the
// theme's widths, the focus ring, and a chip. A theme that rounds its chips into pills shows here first
// (`gallery/shape.rs`).

import DesignSystem
import SwiftUI

struct ShapeSpecimen: View {
    @Environment(\.tokens) private var tokens

    var body: some View {
        let metrics = tokens.metrics
        VStack(alignment: .leading, spacing: Space._4) {
            HStack(spacing: Space._3) {
                box("sm", metrics.radiusSm)
                box("chip", metrics.radiusChip)
                box("control", metrics.radius)
                box("card", metrics.radiusLg)
            }
            HStack(spacing: Space._4) {
                Text("border \(NumberFormat.points(metrics.borderWidth))").font(TextRole.caption.font)
                    .padding(.horizontal, Space._2).frame(height: Metric.control)
                    .overlay(RoundedRectangle(cornerRadius: metrics.radius).strokeBorder(tokens.borderStrong, lineWidth: metrics.borderWidth))
                Rectangle().fill(tokens.divider).frame(width: max(metrics.dividerWidth, 0.0001), height: Metric.control)
                Text("focus \(NumberFormat.points(metrics.focusRingWidth))").font(TextRole.caption.font)
                    .padding(.horizontal, Space._2).frame(height: Metric.control)
                    .overlay(RoundedRectangle(cornerRadius: metrics.radius).strokeBorder(tokens.borderFocus, lineWidth: metrics.focusRingWidth))
                Text("chip").font(TextRole.caption.font).padding(.horizontal, Space._2).frame(height: Metric.badgeHeight)
                    .background(tokens.chipTint, in: RoundedRectangle(cornerRadius: metrics.radiusChip))
                    .overlay(RoundedRectangle(cornerRadius: metrics.radiusChip).strokeBorder(tokens.chipBorder, lineWidth: metrics.borderWidth))
            }
            .padding(Space._1)
        }
    }

    private func box(_ name: String, _ radius: Double) -> some View {
        VStack(spacing: Space._1) {
            RoundedRectangle(cornerRadius: radius).fill(tokens.surfaceElevated)
                .overlay(RoundedRectangle(cornerRadius: radius).strokeBorder(tokens.border, lineWidth: tokens.metrics.borderWidth))
                .frame(width: 56, height: 40)
            Text("\(name) \(NumberFormat.points(radius))").font(TextRole.caption.font).foregroundStyle(tokens.textFaint)
        }
    }
}

enum NumberFormat {
    static func points(_ value: Double) -> String { value == value.rounded() ? String(Int(value)) : String(value) }
}
