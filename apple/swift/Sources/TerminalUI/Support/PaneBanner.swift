// A banner over a pane (`style::banner`): the kind's glyph, one line in label size, and the buttons at the
// trailing end. Tinted fill in the control corners; Increase Contrast draws the kind's outline.

import DesignSystem
import SwiftUI

public enum BannerKind: Sendable, Equatable { case info, warning, danger, success }

struct PaneBanner<Actions: View>: View {
    let kind: BannerKind
    let text: String
    @ViewBuilder var actions: Actions
    @Environment(\.tokens) private var tokens

    var body: some View {
        let shape = RoundedRectangle(cornerRadius: tokens.metrics.radius)
        HStack(spacing: Space._3) {
            Image(systemName: symbol).font(.system(size: Metric.titleGlyph)).foregroundStyle(glyphColor)
            Text(text).font(.system(size: TextRole.label.size)).foregroundStyle(tokens.textPrimary)
                .frame(maxWidth: .infinity, alignment: .leading)
            actions
        }
        .padding(.leading, Space._3).padding(.trailing, Space._2).padding(.vertical, Self.padY)
        .frame(minHeight: Self.minHeight)
        .background(fill, in: shape)
        .overlay { if ThemeStore.shared.increaseContrast { shape.strokeBorder(border, lineWidth: tokens.metrics.borderWidth) } }
        .accessibilityElement(children: .contain)
        .accessibilityLabel(text)
    }

    /// `BANNER_MIN_H` and `BANNER_PAD_Y` (`style/scale.rs`, `style/messages.rs`).
    static var minHeight: CGFloat { 40 }
    private static var padY: CGFloat { 6 }

    private var symbol: String {
        switch kind {
        case .info: "info.circle"
        case .warning, .danger: "exclamationmark.triangle"
        case .success: "checkmark.shield"
        }
    }
    private var glyphColor: Color {
        switch kind {
        case .info: tokens.statusInfo
        case .warning: tokens.statusWarning
        case .danger: tokens.statusDanger
        case .success: tokens.statusSuccess
        }
    }
    private var fill: Color {
        switch kind {
        case .info: tokens.bannerInfoBackground
        case .warning: tokens.bannerWarningBackground
        case .danger: tokens.bannerDangerBackground
        case .success: tokens.bannerSuccessBackground
        }
    }
    private var border: Color {
        switch kind {
        case .info: tokens.bannerInfoBorder
        case .warning: tokens.bannerWarningBorder
        case .danger: tokens.bannerDangerBorder
        case .success: tokens.bannerSuccessBorder
        }
    }
}
