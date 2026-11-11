// A word on a quiet tint of its own colour (`style::status_badge`): a Probe's or a project's state. Increase Contrast,
// where a faint fill is no edge, gives it an outline.

import SwiftUI

public struct StatusBadge: View {
    let text: String
    let color: Color
    @Environment(\.tokens) private var tokens

    /// The tint behind the word (`STATUS_TINT_ALPHA`).
    private static let tint = 0.15

    public init(text: String, color: Color) { (self.text, self.color) = (text, color) }

    public var body: some View {
        Text(text).textRole(.caption).foregroundStyle(color)
            .padding(.horizontal, Space._2).frame(minWidth: Metric.badgeHeight, minHeight: Metric.badgeHeight)
            .background(color.opacity(Self.tint), in: RoundedRectangle(cornerRadius: tokens.metrics.radiusChip))
            .contrastOutline(RoundedRectangle(cornerRadius: tokens.metrics.radiusChip), color: color)
            .fixedSize()
            .marked("badge")
    }
}
