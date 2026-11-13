// What hangs on a row besides its control: the changed dot, the reset arrow and the (i) that opens the
// description in a card (VoiceOver reads the description from the row itself).

import DesignSystem
import SwiftUI

struct ChangedDot: View {
    @Environment(\.tokens) private var tokens
    var body: some View {
        Circle().fill(tokens.primary).frame(width: SettingsMetric.markDot, height: SettingsMetric.markDot)
            .accessibilityHidden(true)
    }
}

struct ResetArrow: View {
    let action: () -> Void
    var body: some View {
        IconButton(symbol: Icon.reset.rawValue, label: LocalizedStringKey(SettingsText.reset), action: action)
    }
}

struct InfoButton: View {
    let text: String
    @State private var open = false
    @Environment(\.tokens) private var tokens

    var body: some View {
        Button { open.toggle() } label: { Image(Icon.info).foregroundStyle(tokens.textFaint) }
            .buttonStyle(.borderless)
            .popover(isPresented: $open, arrowEdge: .bottom) {
                Text(text)
                    .textRole(.label).foregroundStyle(tokens.textPrimary)
                    .frame(width: 280, alignment: .leading).padding(Space._3)
                    .environment(\.tokens, tokens)
            }
            .accessibilityHidden(true)
    }
}

/// A small tag after a row's label (`style::chip`): "Time Sensitive", "Beta".
struct RowChip: View {
    let text: String
    @Environment(\.tokens) private var tokens

    var body: some View {
        Text(text).textRole(.caption, weight: .medium).foregroundStyle(tokens.textMuted)
            .padding(.horizontal, Space._1)
            .background(tokens.controlBackground, in: RoundedRectangle(cornerRadius: tokens.metrics.radiusChip))
    }
}
