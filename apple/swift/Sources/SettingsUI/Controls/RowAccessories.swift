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
        IconButton(symbol: "arrow.counterclockwise", label: LocalizedStringKey(SettingsText.reset), action: action)
    }
}

struct InfoButton: View {
    let text: String
    @State private var open = false
    @Environment(\.tokens) private var tokens

    var body: some View {
        Button { open.toggle() } label: {
            Image(systemName: "info.circle").font(.system(size: Metric.glyph))
                .foregroundStyle(tokens.textFaint).frame(width: Metric.lead, height: Metric.lead)
        }
        .buttonStyle(.plain)
        .popover(isPresented: $open, arrowEdge: .bottom) {
            Text(text)
                .font(.system(size: TextRole.label.size)).foregroundStyle(tokens.textPrimary)
                .frame(width: 280, alignment: .leading).padding(Space._3)
                .environment(\.tokens, tokens)
        }
        .accessibilityHidden(true)
    }
}
