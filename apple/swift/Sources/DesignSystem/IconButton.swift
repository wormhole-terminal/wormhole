// A borderless symbol button with hover and press feedback. The label is required: it is the VoiceOver name and the tooltip.

import SwiftUI

public struct IconButton: View {
    public let symbol: String
    public let label: LocalizedStringKey
    public let action: () -> Void
    @State private var hovering = false

    public init(symbol: String, label: LocalizedStringKey, action: @escaping () -> Void) {
        self.symbol = symbol
        self.label = label
        self.action = action
    }

    public var body: some View {
        Button(action: action) {
            Image(systemName: symbol)
                .font(.system(size: Metric.glyph, weight: .medium))
                .frame(width: Metric.control, height: Metric.control)
        }
        .buttonStyle(IconButtonStyle(hovering: hovering))
        .onHover { hovering = $0 }
        .help(label)
        .accessibilityLabel(label)
    }
}

private struct IconButtonStyle: ButtonStyle {
    let hovering: Bool

    func makeBody(configuration: Configuration) -> some View {
        let shape = RoundedRectangle(cornerRadius: Radius.row)
        return configuration.label
            .foregroundStyle(hovering || configuration.isPressed ? Palette.textPrimary : Palette.textSecondary)
            .background(configuration.isPressed ? Palette.pressed : hovering ? Palette.hover : .clear, in: shape)
            .contentShape(shape)
            .pressed(configuration.isPressed)
            .motion(Motion.press, value: hovering)
    }
}
