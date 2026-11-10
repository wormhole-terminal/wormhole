// A small count pill: unread output, a section's size. Capped like the GPUI app's (`count_label`).

import SwiftUI

public struct CountBadge: View {
    public static let cap = 999
    public let count: Int
    @Environment(\.tokens) private var tokens

    public init(_ count: Int) { self.count = count }

    /// The number, or "999+" past the cap.
    public static func label(_ count: Int) -> String { count > cap ? "\(cap)+" : "\(count)" }

    public var body: some View {
        Text(Self.label(count))
            .font(TextRole.caption.font.weight(.semibold).monospacedDigit())
            .foregroundStyle(tokens.textPrimary)
            .padding(.horizontal, Space._1)
            // At least a square, so a lone digit sits in a circle-ish pill.
            .frame(minWidth: Metric.badgeHeight, minHeight: Metric.badgeHeight)
            .background(tokens.border, in: Capsule())
            // Digits roll to the new number instead of swapping.
            .contentTransition(.numericText(value: Double(count)))
            .motion(Motion.selection, value: count)
            .accessibilityHidden(true)
    }
}
