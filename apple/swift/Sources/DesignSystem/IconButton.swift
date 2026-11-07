// A borderless symbol button with a hover fill. The label is required: it is the VoiceOver name and the tooltip.

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
                .foregroundStyle(hovering ? Palette.textPrimary : Palette.textSecondary)
                .frame(width: Metric.control, height: Metric.control)
                .background(hovering ? Palette.hover : .clear, in: RoundedRectangle(cornerRadius: Radius.row))
                .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .onHover { hovering = $0 }
        .help(label)
        .accessibilityLabel(label)
    }
}
