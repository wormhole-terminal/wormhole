// One theme in miniature: a narrow sidebar band, five lines in the theme's ANSI colours on its terminal
// background, and a band with its name. The current pick has the theme's own primary ring and a check.

import DesignSystem
import SwiftUI

struct ThemeCard: View {
    let theme: Theme
    let isSelected: Bool
    let action: () -> Void
    @Environment(\.tokens) private var tokens
    @State private var hovering = false

    private static let sceneHeight: CGFloat = 88
    private static let ring: CGFloat = 2
    private static let sidebarWidth: CGFloat = 40

    var body: some View {
        let own = theme.tokens()
        let shape = RoundedRectangle(cornerRadius: tokens.metrics.radiusLg)
        Button(action: action) {
            VStack(spacing: 0) {
                HStack(spacing: 0) {
                    VStack(alignment: .leading, spacing: Space._1) {
                        Capsule().fill(own.sidebarRowSelected).frame(width: 20, height: 6)
                        Capsule().fill(own.textMuted).frame(width: 14, height: 3)
                        Capsule().fill(own.textMuted).frame(width: 14, height: 3)
                    }
                    .padding(Space._2).frame(width: Self.sidebarWidth, alignment: .topLeading).frame(maxHeight: .infinity, alignment: .top)
                    .background(own.sidebarBackground)
                    VStack(alignment: .leading, spacing: Space._1) {
                        ForEach([(4, 0.6), (2, 0.8), (3, 0.45), (7, 0.7), (1, 0.3)], id: \.0) { index, width in
                            Capsule().fill(own.terminal.ansi[index].color).frame(height: 3).scaleEffect(x: width, anchor: .leading)
                        }
                    }
                    .padding(Space._2).frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
                    .background(own.terminal.background.color)
                }
                .frame(height: Self.sceneHeight)
                HStack {
                    Text(theme.name).font(.system(size: TextRole.label.size, weight: .medium)).lineLimit(1)
                        .foregroundStyle(isSelected ? own.primary : own.textPrimary)
                    Spacer(minLength: 0)
                    if isSelected { Image(Icon.check).font(.system(size: Metric.glyph, weight: .semibold)).foregroundStyle(own.primary) }
                }
                .padding(.horizontal, Space._3).frame(height: Metric.control + Space._1).background(own.surfacePanel)
            }
            .clipShape(shape)
            .overlay(shape.strokeBorder(isSelected ? own.primary : (hovering ? tokens.borderStrong : .clear), lineWidth: Self.ring))
            .contentShape(shape)
        }
        .buttonStyle(.plain)
        .onHover { hovering = $0 }
        .motion(Motion.press, value: hovering)
        .accessibilityLabel(theme.name)
        .accessibilityAddTraits(isSelected ? [.isButton, .isSelected] : .isButton)
    }
}
