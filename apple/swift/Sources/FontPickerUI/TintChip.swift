// A small tinted tag (`tint_chip`): "Bundled" beside the font's name in a select.

import DesignSystem
import SwiftUI

public struct TintChip: View {
    let text: String
    @Environment(\.tokens) private var tokens

    public init(_ text: String) { self.text = text }

    public var body: some View {
        Text(text).textRole(.caption).foregroundStyle(tokens.textMuted)
            .padding(.horizontal, Space._2).frame(height: Metric.badgeHeight + Space._1)
            .background(tokens.chipTint, in: RoundedRectangle(cornerRadius: Radius.chip))
    }
}
