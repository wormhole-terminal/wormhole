// What a theme's metrics shape: the four corner roles and the focus ring as boxes that select their metric when
// clicked, every chip-shaped component side by side, a hairline and a column divider at the theme's widths, and the
// focus ring round a control. A theme that rounds its chips into pills shows here first (`gallery/shape.rs`).
// text-file: sample content of the Theme Studio gallery; the GPUI gallery shows its specimens untranslated too (studio/gallery)

import DesignSystem
import SwiftUI

struct ShapeSpecimen: View {
    let context: GalleryContext
    @State private var layout = "split"
    @Environment(\.tokens) private var tokens

    var body: some View {
        let metrics = tokens.metrics
        VStack(alignment: .leading, spacing: Space._4) {
            HStack(spacing: Space._3) {
                box("radius_sm", metrics.radiusSm)
                box("radius_chip", metrics.radiusChip)
                box("radius", metrics.radius)
                box("radius_lg", metrics.radiusLg)
                box("focus_ring_width", metrics.radius, ring: metrics.focusRingWidth)
            }
            HStack(spacing: Space._2) {
                Text("Tab").font(TextRole.caption.font).padding(.horizontal, Space._2).frame(height: Metric.badgeHeight)
                    .background(tokens.chipSelected, in: RoundedRectangle(cornerRadius: metrics.radiusChip)).foregroundStyle(tokens.textOnSelected)
                CountBadge(3)
                StatusBadge(text: "badge", color: tokens.statusSuccessText)
                KeyCap(text: "⌘K")
                Segmented([("Split", "split"), ("Unified", "unified")], selection: $layout)
            }
            HStack(spacing: Space._4) {
                Text("border \(NumberFormat.points(metrics.borderWidth))").font(TextRole.caption.font)
                    .padding(.horizontal, Space._2).frame(height: Metric.control)
                    .overlay(RoundedRectangle(cornerRadius: metrics.radius).strokeBorder(tokens.borderStrong, lineWidth: metrics.borderWidth))
                Rectangle().fill(tokens.divider).frame(width: max(metrics.dividerWidth, 0.0001), height: Metric.control)
                Text("focus \(NumberFormat.points(metrics.focusRingWidth))").font(TextRole.caption.font)
                    .padding(.horizontal, Space._2).frame(height: Metric.control)
                    .overlay(RoundedRectangle(cornerRadius: metrics.radius).strokeBorder(tokens.borderFocus, lineWidth: metrics.focusRingWidth))
                    .marked("focus.ring")
                Text("chip").font(TextRole.caption.font).padding(.horizontal, Space._2).frame(height: Metric.badgeHeight)
                    .background(tokens.chipTint, in: RoundedRectangle(cornerRadius: metrics.radiusChip))
                    .overlay(RoundedRectangle(cornerRadius: metrics.radiusChip).strokeBorder(tokens.chipBorder, lineWidth: metrics.borderWidth))
            }
            .padding(Space._1)
        }
    }

    /// A metric's sample box, in the primary while it is the token being edited (the border colour otherwise), or with a
    /// `ring` that wide in the focus colour; a click selects the metric (`metric_sample`).
    private func box(_ key: String, _ radius: Double, ring: Double? = nil) -> some View {
        let name = "metrics.\(key)"
        let selected = context.selected == name
        let shape = RoundedRectangle(cornerRadius: radius)
        return Button { context.select(name) } label: {
            VStack(spacing: Space._1) {
                shape.fill(tokens.stateHover)
                    .overlay(shape.strokeBorder(ring == nil ? (selected ? tokens.primary : tokens.border) : tokens.borderFocus,
                                                lineWidth: ring ?? tokens.metrics.borderWidth))
                    .frame(width: Metric.control * 2, height: Metric.control * 2)
                Text("\(key) \(NumberFormat.points(ring ?? radius))").font(TextRole.caption.font)
                    .foregroundStyle(selected ? tokens.textPrimary : tokens.textFaint)
            }
        }
        .buttonStyle(.plain).accessibilityLabel(name)
    }
}

enum NumberFormat {
    static func points(_ value: Double) -> String { value == value.rounded() ? String(Int(value)) : String(value) }
}
