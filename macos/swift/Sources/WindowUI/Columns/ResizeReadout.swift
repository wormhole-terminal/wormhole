// The readout beside a dragged column edge (`style::readout`): tooltip colours, digits that keep their width. The
// window places it at the pointer (`WindowBody`).

import DesignSystem
import SwiftUI

/// The words beside the pointer while a column is dragged: its width, and why it stopped ("430 · max") or what
/// letting go does ("Close").
struct ResizeReadout: View {
    let text: String
    let pointer: CGFloat
    let pointerY: CGFloat
    @Environment(\.chrome) private var colours

    var body: some View {
        Text(text)
            .font(TextRole.caption.font.weight(.medium))
            .monospacedDigit()
            .foregroundStyle(colours.tooltipText)
            .padding(.horizontal, Space._2)
            .frame(height: Chrome.hit - Space._1)
            .background(colours.tooltip, in: RoundedRectangle(cornerRadius: colours.radius))
            .marked("readout")
            .offset(x: pointer + Space._2, y: pointerY - Space._2)
            .allowsHitTesting(false)
            .accessibilityHidden(true)
    }
}
