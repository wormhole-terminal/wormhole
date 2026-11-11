// The one-line rules between the Studio's regions, in the theme's border colour and width (`rule_top`,
// `rule_bottom`, `rule_left` of the GPUI app's style).

import DesignSystem
import SwiftUI

enum HairlineEdge { case top, bottom, leading }

private struct Hairline: ViewModifier {
    let edge: HairlineEdge
    @Environment(\.tokens) private var tokens

    func body(content: Content) -> some View {
        let width = CGFloat(tokens.metrics.borderWidth)
        content.overlay(alignment: edge == .top ? .top : edge == .bottom ? .bottom : .leading) {
            Rectangle().fill(tokens.border)
                .frame(width: edge == .leading ? width : nil, height: edge == .leading ? nil : width)
                .allowsHitTesting(false)
        }
    }
}

extension View {
    func hairline(_ edge: HairlineEdge) -> some View { modifier(Hairline(edge: edge)) }
}
