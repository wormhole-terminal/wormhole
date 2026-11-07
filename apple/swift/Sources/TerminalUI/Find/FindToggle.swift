// One of the field's three toggles: a small square that lights with the selected tint while on. A real
// `Toggle`, so VoiceOver says "Match case, on" and Space flips it.

import DesignSystem
import SwiftUI

struct FindToggle: View {
    let option: FindOption
    let model: FindModel
    @Environment(\.tokens) private var tokens
    @State private var hovering = false

    var body: some View {
        let on = model.options[option]
        Toggle(isOn: Binding(get: { on }, set: { _ in model.toggle(option) })) {
            glyph
                .foregroundStyle(on ? tokens.textPrimary : tokens.textMuted)
                .frame(width: Metric.controlSmall, height: Metric.controlSmall)
        }
        .toggleStyle(.button)
        .buttonStyle(.plain)
        .background(on ? tokens.stateSelected : hovering ? tokens.stateHover : .clear,
                    in: RoundedRectangle(cornerRadius: tokens.metrics.radiusSm))
        .onHover { hovering = $0 }
        .motion(Motion.press, value: on)
        .help(L10n.findOption(option))
        .accessibilityLabel(L10n.findOption(option))
    }

    /// The marks of the lucide icons the GPUI bar uses: "Aa", a word with its underline, ".*".
    @ViewBuilder private var glyph: some View {
        switch option {
        case .matchCase: Text("Aa").font(.system(size: TextRole.caption.size, weight: .medium))
        case .wholeWord: Text("ab").underline().font(.system(size: TextRole.caption.size, weight: .medium))
        case .regex: Text(".*").font(.system(size: TextRole.caption.size, weight: .medium, design: .monospaced))
        }
    }
}
