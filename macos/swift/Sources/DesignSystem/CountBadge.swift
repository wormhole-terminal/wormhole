// The one count badge (`count_badge`, `selected_count_badge`): unread output, a section's size, a band's count, a folded
// group's members. Chip corners like every badge of the GPUI app (`badge`: `radius_chip`), at least as wide as it is tall,
// the digits in `text.primary` 600 on the hairline colour. Capped like the GPUI app's (`count_label`).

import SwiftUI

public struct CountBadge: View {
    public static let cap = 999
    let text: String
    /// The number the digits roll to; `nil` for a count spelled by its caller (grouped digits).
    let value: Int?
    /// On a selected tint (the active icon tab, the current session): a stronger tint of the primary with the tint's text colour (`selected_count_badge`).
    public let selected: Bool
    @Environment(\.tokens) private var tokens

    /// Where the view that asked for it was built, for inspect mode.
    let file: StaticString
    let line: UInt

    public init(_ count: Int, selected: Bool = false, file: StaticString = #fileID, line: UInt = #line) {
        (self.text, self.value, self.selected, self.file, self.line) = (Self.label(count), count, selected, file, line)
    }

    /// A count its caller has already spelled ("1,234" in a panel band).
    public init(text: String, selected: Bool = false, file: StaticString = #fileID, line: UInt = #line) {
        (self.text, self.value, self.selected, self.file, self.line) = (text, nil, selected, file, line)
    }

    /// The number, or "999+" past the cap.
    public static func label(_ count: Int) -> String { count > cap ? "\(cap)+" : "\(count)" }

    public var body: some View {
        Text(text)
            .font(tokens.font(.caption).weight(.semibold).monospacedDigit())
            .lineLimit(1)
            .fixedSize()
            .foregroundStyle(selected ? tokens.sidebarRowSelectedText : tokens.textPrimary)
            .padding(.horizontal, Space._1)
            // At least a square, so a lone digit sits in a square chip.
            .frame(minWidth: Metric.badgeHeight, minHeight: Metric.badgeHeight)
            .background(selected ? tokens.stateSelectedInset : tokens.border, in: RoundedRectangle(cornerRadius: tokens.metrics.radiusChip))
            .marked("badge", style: "padding x _1 · min badgeHeight · radius chip · text caption", file: file, line: line)  // text: inspect mode's note, in the scales' names
            // Digits roll to the new number instead of swapping.
            .contentTransition(value.map { ContentTransition.numericText(value: Double($0)) } ?? .numericText())
            .motion(Motion.selection, value: text)
            .accessibilityHidden(true)
    }
}
