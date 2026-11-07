// A notice that asks to be read (`style::banner`): the kind's tint, its icon and the text. A warning is a problem that
// leaves things as they were, a danger one that stopped what the person asked for.

import DesignSystem
import SwiftUI

struct Banner<Accessory: View>: View {
    enum Kind { case info, warning, danger }
    let kind: Kind
    let text: String
    /// At the right: a button that goes with the notice (a preview page's way to the feature flags).
    @ViewBuilder let accessory: Accessory
    @Environment(\.tokens) private var tokens

    var body: some View {
        HStack(spacing: Space._3) {
            Image(systemName: kind == .info ? "info.circle.fill" : "exclamationmark.triangle.fill").font(.system(size: Metric.glyph)).foregroundStyle(accent)
            Text(text).font(.system(size: TextRole.label.size)).foregroundStyle(tokens.textPrimary)
                .frame(maxWidth: .infinity, alignment: .leading).fixedSize(horizontal: false, vertical: true)
            accessory
        }
        .padding(.horizontal, Space._3).padding(.vertical, Space._2)
        .background(fill, in: RoundedRectangle(cornerRadius: tokens.metrics.radius))
        .overlay {
            if ThemeStore.shared.increaseContrast {
                RoundedRectangle(cornerRadius: tokens.metrics.radius).strokeBorder(border, lineWidth: tokens.metrics.borderWidth)
            }
        }
        .accessibilityElement(children: .combine)
    }

    private var accent: Color {
        switch kind { case .info: tokens.statusInfo; case .warning: tokens.statusWarning; case .danger: tokens.statusDanger }
    }
    private var fill: Color {
        switch kind { case .info: tokens.bannerInfoBackground; case .warning: tokens.bannerWarningBackground; case .danger: tokens.bannerDangerBackground }
    }
    private var border: Color {
        switch kind { case .info: tokens.bannerInfoBorder; case .warning: tokens.bannerWarningBorder; case .danger: tokens.bannerDangerBorder }
    }
}

extension Banner where Accessory == EmptyView {
    init(kind: Kind, text: String) { self.init(kind: kind, text: text) { EmptyView() } }
}
