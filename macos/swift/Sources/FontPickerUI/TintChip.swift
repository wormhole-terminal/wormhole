// A small tinted tag (`tint_chip`): "Bundled" beside the font's name in a select.

import DesignSystem
import SwiftUI

public struct TintChip: View {
    let text: String
    @Environment(\.tokens) private var tokens

    public init(_ text: String) { self.text = text }

    public var body: some View {
        Chip(fill: tokens.chipTint, text: tokens.textMuted, height: Metric.badgeHeight + Space._1) { Text(text).textRole(.caption) }
            .marked("tint.chip")
    }
}
