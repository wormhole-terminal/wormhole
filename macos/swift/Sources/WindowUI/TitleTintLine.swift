// The title row's group colour under Increase Contrast: a tint behind the row says little to someone who needs the contrast,
// so the colour is a 2 pt line along the bottom of the row's rounded bar instead (`window/title`: `rule_bottom_in(…, 2.0)`).

import DesignSystem
import PanesModel
import SidebarUI
import SwiftUI
import WorkspaceModel

struct TitleTintLine: View {
    let color: GroupColor?
    @Environment(\.tokens) private var tokens
    @Environment(\.chrome) private var colours
    @Environment(WindowLook.self) private var look

    var body: some View {
        if look.themes.increaseContrast, let color {
            Color.clear
                .overlay(alignment: .bottom) { Rectangle().fill(color.titleLine(tokens).color).frame(height: Self.thickness) }
                .clipShape(RoundedRectangle(cornerRadius: colours.radiusLarge))
                .padding(.horizontal, TitleMetrics.barInset)
                .padding(.vertical, TitleMetrics.barInset)
                .allowsHitTesting(false)
                .accessibilityHidden(true)
        }
    }

    /// The line's thickness (`rule_bottom_in(…, 2.0)`).
    private static var thickness: CGFloat { 2 }
}
