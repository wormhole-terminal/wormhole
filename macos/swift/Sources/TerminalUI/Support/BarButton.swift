// The small outlined button of a pane bar (`Button::small().outline()`): control corners, a strong outline,
// the label in body-minus text; hover lifts the fill, press sinks it.

import DesignSystem
import SwiftUI

struct BarButton: View {
    let title: String
    var symbol: String? = nil
    var help: String? = nil
    let action: () -> Void
    @Environment(\.tokens) private var tokens
    @State private var hovering = false

    var body: some View {
        Button(action: action) {
            HStack(spacing: Space._1) {
                if let symbol { Image(systemName: symbol).font(.system(size: tokens.size(.caption))) }
                Text(title).font(.system(size: tokens.size(.label)))
            }
            .padding(.horizontal, Space._2)
            .frame(height: Metric.titleControl)
        }
        .buttonStyle(BarButtonStyle(tokens: tokens, hovering: hovering))
        .onHover { hovering = $0 }
        .help(help ?? title)
    }
}

private struct BarButtonStyle: ButtonStyle {
    let tokens: ThemeTokens
    let hovering: Bool

    func makeBody(configuration: Configuration) -> some View {
        let shape = RoundedRectangle(cornerRadius: tokens.metrics.radius)
        configuration.label
            .foregroundStyle(tokens.textPrimary)
            .background(configuration.isPressed ? tokens.stateActive : hovering ? tokens.stateHover : .clear, in: shape)
            .overlay { shape.strokeBorder(tokens.borderStrong, lineWidth: tokens.metrics.borderWidth) }
            .contentShape(shape)
            .pressed(configuration.isPressed)
            .motion(Motion.press, value: hovering)
    }
}
