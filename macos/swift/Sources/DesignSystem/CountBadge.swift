// A small count pill: unread output, a section's size. Capped like the GPUI app's (`count_label`).

import SwiftUI

public struct CountBadge: View {
    public static let cap = 999
    public let count: Int
    /// On a selected tint (the active icon tab, the current session): a stronger tint of the primary with the tint's text colour (`selected_count_badge`).
    public let selected: Bool
    @Environment(\.tokens) private var tokens

    /// Where the view that asked for it was built, for inspect mode.
    let file: StaticString
    let line: UInt

    public init(_ count: Int, selected: Bool = false, file: StaticString = #fileID, line: UInt = #line) {
        (self.count, self.selected, self.file, self.line) = (count, selected, file, line)
    }

    /// The number, or "999+" past the cap.
    public static func label(_ count: Int) -> String { count > cap ? "\(cap)+" : "\(count)" }

    public var body: some View {
        Text(Self.label(count))
            .font(tokens.font(.caption).weight(.semibold).monospacedDigit())
            .foregroundStyle(selected ? tokens.sidebarRowSelectedText : tokens.textPrimary)
            .padding(.horizontal, Space._1)
            // At least a square, so a lone digit sits in a circle-ish pill.
            .frame(minWidth: Metric.badgeHeight, minHeight: Metric.badgeHeight)
            .background(selected ? tokens.stateSelectedInset : tokens.border, in: Capsule())
            .marked("badge", style: "padding x _1 · min badgeHeight · radius full · text caption", file: file, line: line)  // text: inspect mode's note, in the scales' names
            // Digits roll to the new number instead of swapping.
            .contentTransition(.numericText(value: Double(count)))
            .motion(Motion.selection, value: count)
            .accessibilityHidden(true)
    }
}
