// A theme as the Look step offers it: a sidebar strip and a terminal in the theme's own colours, its name and a
// check on the current one (`look_step::theme_card`). Drawn from the theme's tokens, not the window's.

import DesignSystem
import SwiftUI

struct ThemeCard: View {
    let theme: Theme
    let current: Bool
    let action: () -> Void
    @Environment(\.tokens) private var window
    @State private var hovering = false

    /// The sidebar strip's share of the preview (`relative(0.3)`).
    private static let sidebarShare: CGFloat = 0.3

    var body: some View {
        let tokens = theme.tokens()
        let shape = RoundedRectangle(cornerRadius: window.metrics.radius)
        Button(action: action) {
            VStack(spacing: 0) {
                preview(tokens)
                HStack(spacing: Space._1) {
                    Text(theme.name).font(TextRole.caption.font.weight(current ? .semibold : .medium)).lineLimit(1)
                    Spacer(minLength: 0)
                    if current { Image(systemName: "checkmark").font(.system(size: Metric.glyph)) }
                    // A light theme that is not the pick says so with a sun (`look_step::theme_card`).
                    else if theme.appearance == .light { Image(systemName: "sun.max").font(.system(size: Metric.glyph)).foregroundStyle(window.textMuted) }
                }
                .foregroundStyle(current ? tokens.primary : tokens.textPrimary)
                .padding(.horizontal, Space._2).frame(height: OnboardingMetrics.cardStrip)
                .background(tokens.surfacePanel)
            }
            .frame(height: OnboardingMetrics.cardHeight)
            .clipShape(shape)
            .overlay {
                shape.strokeBorder(
                    current ? window.primary : window.borderStrong,
                    lineWidth: current ? window.metrics.focusRingWidth : window.metrics.borderWidth)
            }
            .opacity(hovering ? 0.92 : 1)
        }
        .buttonStyle(.plain)
        .onHover { hovering = $0 }
        .accessibilityLabel(theme.name)
        .accessibilityAddTraits(current ? .isSelected : [])
    }

    private func preview(_ tokens: ThemeTokens) -> some View {
        GeometryReader { proxy in
            let side = proxy.size.width * Self.sidebarShare
            let main = proxy.size.width - side
            HStack(spacing: 0) {
                column([(tokens.textMuted, 0.8), (tokens.textMuted, 0.6)], width: side)
                    .background(tokens.sidebarBackground)
                column(
                    [(tokens.primary, 0.7), (tokens.terminal.ansi[2].color, 0.85), (tokens.terminal.ansi[3].color, 0.5)],
                    width: main
                )
                .background(tokens.terminal.background.color)
            }
        }
    }

    private func column(_ bars: [(Color, CGFloat)], width: CGFloat) -> some View {
        VStack(alignment: .leading, spacing: OnboardingMetrics.previewGap) {
            ForEach(Array(bars.enumerated()), id: \.offset) { _, bar in
                Capsule().fill(bar.0).frame(width: (width - 2 * OnboardingMetrics.previewPad) * bar.1, height: OnboardingMetrics.previewBar)
            }
            Spacer(minLength: 0)
        }
        .padding(OnboardingMetrics.previewPad)
        .frame(width: width, alignment: .topLeading)
    }
}
