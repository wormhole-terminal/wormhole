// The pill a readout is written on (`style::messages::readout_pill`): tooltip colours, caption text with
// digits that keep their width. The block readout and the size readout are both this.

import DesignSystem
import SwiftUI

struct ReadoutPill<Content: View>: View {
    var role: TextRole = .caption
    var cornerRadius: CGFloat? = nil
    @ViewBuilder var content: Content
    @Environment(\.tokens) private var tokens

    var body: some View {
        content
            .font(.system(size: tokens.size(role)))
            .monospacedDigit()
            .foregroundStyle(tokens.tooltipText)
            .lineLimit(1)
            .fixedSize()
            .background(tokens.tooltipBackground, in: RoundedRectangle(cornerRadius: cornerRadius ?? tokens.metrics.radius))
    }
}
