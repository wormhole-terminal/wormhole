// A title row's icon button: a 28 pt hit with a 24 pt face and a 16 pt glyph, a `band.hover` wash under the
// pointer, a tooltip that is also the VoiceOver name (`style::icon_button(...).in_title_row()`).

import DesignSystem
import SwiftUI

struct TitleIconButton<Glyph: View>: View {
    let label: String
    var enabled = true
    let action: () -> Void
    @ViewBuilder let glyph: () -> Glyph
    @State private var hovering = false
    @Environment(\.tokens) private var tokens

    var body: some View {
        Button(action: action) {
            glyph()
                .frame(width: Metric.titleControl, height: Metric.titleControl)
                .foregroundStyle(hovering ? tokens.textPrimary : tokens.textMuted)
                .background(hovering ? tokens.bandHover : .clear, in: RoundedRectangle(cornerRadius: tokens.metrics.radiusSm))
                .frame(width: Metric.hitMin, height: Metric.hitMin)
                .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .disabled(!enabled)
        .opacity(enabled ? 1 : 0.4)
        .onHover { hovering = $0 }
        .help(label)
        .accessibilityLabel(label)
    }
}

/// A symbol at the title row's glyph size.
struct SymbolGlyph: View {
    let name: String

    var body: some View { Image(systemName: name).font(.system(size: Metric.titleGlyph, weight: .regular)) }
}

extension TitleIconButton where Glyph == SymbolGlyph {
    init(symbol: String, label: String, enabled: Bool = true, action: @escaping () -> Void) {
        self.init(label: label, enabled: enabled, action: action) { SymbolGlyph(name: symbol) }
    }
}
