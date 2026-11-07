// A row's button that does something (open a folder, put things back): an outlined flat button with an icon,
// and the link-coloured text button that takes you to another page (`style::button`, `style::link_button`).

import DesignSystem
import SwiftUI

struct ActionButton: View {
    let title: String
    let symbol: String?
    var isDestructive = false
    let action: () -> Void
    @Environment(\.tokens) private var tokens
    @State private var hovering = false

    var body: some View {
        Button(action: action) {
            HStack(spacing: Space._2) {
                if let symbol { Image(systemName: symbol).font(.system(size: Metric.glyph)) }
                Text(title).font(.system(size: TextRole.label.size, weight: .medium))
            }
            .padding(.horizontal, Space._3)
            .frame(height: Metric.control)
            .foregroundStyle(isDestructive ? tokens.statusDangerText : tokens.buttonText)
            .background(hovering ? tokens.buttonHover : tokens.buttonBackground, in: RoundedRectangle(cornerRadius: tokens.metrics.radius))
            .overlay(RoundedRectangle(cornerRadius: tokens.metrics.radius).strokeBorder(tokens.border, lineWidth: tokens.metrics.borderWidth))
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .onHover { hovering = $0 }
        .motion(Motion.press, value: hovering)
    }
}

struct LinkButton: View {
    let title: String
    var goes = false
    let action: () -> Void
    @Environment(\.tokens) private var tokens
    @State private var hovering = false

    var body: some View {
        Button(action: action) {
            HStack(spacing: Space._1) {
                Text(title)
                if goes { Image(systemName: "arrow.right").font(.system(size: Metric.chevron, weight: .semibold)) }
            }
            .font(.system(size: TextRole.label.size, weight: .medium))
            .foregroundStyle(tokens.textLink)
            .padding(.horizontal, Space._2)
            .frame(height: Metric.control)
            .background(hovering ? tokens.stateHover : .clear, in: RoundedRectangle(cornerRadius: tokens.metrics.radiusChip))
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .onHover { hovering = $0 }
    }
}
