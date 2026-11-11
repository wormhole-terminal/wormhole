// A borderless symbol button with hover and press feedback. The label is required: it is the VoiceOver name and the tooltip.

import SwiftUI

public struct IconButton: View {
    public let symbol: String
    /// The Lucide glyph GPUI's button draws, where one is generated; otherwise `symbol`'s SF Symbol.
    public var lucide: Lucide?
    public let label: LocalizedStringKey
    public let action: () -> Void
    @State private var hovering = false

    public init(symbol: String, label: LocalizedStringKey, action: @escaping () -> Void) {
        self.symbol = symbol
        self.label = label
        self.action = action
    }

    public init(lucide: Lucide, label: LocalizedStringKey, action: @escaping () -> Void) {
        self.init(symbol: "", label: label, action: action)
        self.lucide = lucide
    }

    public var body: some View {
        Button(action: action) {
            Group {
                if let lucide { LucideView(icon: lucide, size: Metric.glyph) } else { Image(systemName: symbol).font(.system(size: Metric.glyph, weight: .medium)) }
            }
            .frame(width: Metric.control, height: Metric.control)
        }
        .buttonStyle(IconButtonStyle(hovering: hovering))
        .marked("icon.button")
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
