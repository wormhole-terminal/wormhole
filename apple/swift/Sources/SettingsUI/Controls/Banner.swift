// A notice that asks to be read (`style::banner`): the kind's tint, its icon and the text. A warning is a problem that
// leaves things as they were, a danger one that stopped what the person asked for.

import DesignSystem
import SwiftUI

struct Banner: View {
    enum Kind { case warning, danger }
    let kind: Kind
    let text: String
    @Environment(\.tokens) private var tokens

    var body: some View {
        HStack(spacing: Space._3) {
            Image(systemName: "exclamationmark.triangle.fill").font(.system(size: Metric.glyph)).foregroundStyle(accent)
            Text(text).font(.system(size: TextRole.label.size)).foregroundStyle(tokens.textPrimary)
                .frame(maxWidth: .infinity, alignment: .leading).fixedSize(horizontal: false, vertical: true)
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

    private var accent: Color { kind == .warning ? tokens.statusWarning : tokens.statusDanger }
    private var fill: Color { kind == .warning ? tokens.bannerWarningBackground : tokens.bannerDangerBackground }
    private var border: Color { kind == .warning ? tokens.bannerWarningBorder : tokens.bannerDangerBorder }
}
