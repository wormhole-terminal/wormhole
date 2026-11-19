// What hangs on a row besides its control: the changed dot, the reset arrow and the (i) that opens the
// description in a card (VoiceOver reads the description from the row itself).

import DesignSystem
import SwiftUI
import WordsModel

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
        IconButton(symbol: Icon.reset.rawValue, label: SettingsText.reset, action: action)
    }
}

/// The (i) after a row's label (`info.rs`): a 24 pt chip-cornered face, faint at rest, the text colour on the hover fill or
/// while its card is open; a button VoiceOver reaches by itself ("More information"), whose card reads the description.
struct InfoButton: View {
    let text: String
    @State private var open = false
    @State private var hovering = false
    @Environment(\.tokens) private var tokens

    var body: some View {
        let shape = RoundedRectangle(cornerRadius: tokens.metrics.radiusChip)
        Button { open.toggle() } label: {
            Image(Icon.info).foregroundStyle(open || hovering ? tokens.textPrimary : tokens.textFaint)
                .frame(width: Metric.titleControl, height: Metric.titleControl)
                .background(hovering ? tokens.stateHover : .clear, in: shape)
                .contentShape(shape)
        }
        .buttonStyle(.plain)
        .onHover { hovering = $0 }
        .popover(isPresented: $open, arrowEdge: .bottom) {
            Text(text)
                .textRole(.label).foregroundStyle(tokens.textPrimary)
                .frame(width: 280, alignment: .leading).padding(Space._3)
                .environment(\.tokens, tokens)
        }
        .accessibilityLabel(Words.text("settings-more-information", fallback: "More information"))
    }
}

/// A small tag after a row's label (`style::chip`): "Time Sensitive", "Beta".
struct RowChip: View {
    let text: String
    @Environment(\.tokens) private var tokens

    var body: some View {
        Chip(fill: tokens.controlBackground, text: tokens.textMuted, leading: Space._1) { Text(text).textRole(.caption, weight: .medium) }
    }
}
