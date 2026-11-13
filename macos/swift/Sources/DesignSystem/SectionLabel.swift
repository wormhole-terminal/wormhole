// A section's heading: quiet caption text, with a disclosure chevron when the section can fold.

import SwiftUI

public struct SectionLabel: View {
    @Environment(\.tokens) private var tokens
    public let title: String
    /// `nil`: not foldable, no chevron.
    public let isCollapsed: Bool?

    public init(_ title: String, isCollapsed: Bool? = nil) {
        self.title = title
        self.isCollapsed = isCollapsed
    }

    public var body: some View {
        HStack(spacing: Space._2) {
            if let isCollapsed {
                Image(systemName: "chevron.right")
                    .font(.system(size: Metric.chevron, weight: .bold))
                    .rotationEffect(.degrees(isCollapsed ? 0 : 90))
                    .frame(width: Metric.lead)
                    .motion(Motion.layout, value: isCollapsed)
            }
            // A caption over a group of rows is capitals (`section_label`); a foldable band keeps its case.
            Text(title).textCase(isCollapsed == nil ? .uppercase : nil).font(tokens.font(.caption).weight(.semibold))
            Spacer(minLength: 0)
        }
        .foregroundStyle(tokens.textMuted)
        .frame(minHeight: Metric.control)
    }
}
