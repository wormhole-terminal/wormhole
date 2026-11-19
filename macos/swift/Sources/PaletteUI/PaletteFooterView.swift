// The footer along a palette's bottom (`style::palette_footer`): the keys that work on what is selected, and a
// count or a hint at the right, in muted caption text, a hairline over it.

import DesignSystem
import PaletteModel
import SwiftUI
import WordsModel

struct PaletteFooterView: View {
    let hints: [KeyHint]
    let trailing: PaletteFooter.Trailing
    @Environment(\.tokens) private var tokens

    var body: some View {
        HStack(spacing: Space._4) {
            ForEach(hints, id: \.keys) { KeyHintView(keys: $0.keys, label: $0.label) }
            Spacer(minLength: 0)
            trailingText
        }
        .font(.system(size: tokens.size(.caption)))
        .foregroundStyle(tokens.textMuted)
        .padding(.horizontal, Space._4)
        .frame(height: Metric.paletteRowHeight)
        .overlay(alignment: .top) { Rule(color: tokens.border, width: tokens.metrics.borderWidth) }
    }

    @ViewBuilder private var trailingText: some View {
        switch trailing {
        case .count(let n):
            Text(Words.text("palette-results", ["count": .whole(n)], fallback: n == 1 ? "1 result" : "\(n) results"))
        case .actionsHint:
            HStack(spacing: Space._1) {
                Text(Words.text("palette-actions-hint-type", fallback: "Type"))
                Text(">").font(.system(size: tokens.size(.caption), design: .monospaced)).foregroundStyle(tokens.textPrimary)
                Text(Words.text("palette-actions-hint-for-actions", fallback: "for actions"))
            }
        case .none: EmptyView()
        }
    }
}
