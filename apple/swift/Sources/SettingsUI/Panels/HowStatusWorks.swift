// "How status works" (`draw_how`): what an agent's status is with hooks and without, in two columns, and that the hooks
// do nothing outside Wormhole.

import DesignSystem
import SwiftUI
import WordsModel

struct HowStatusWorks: View {
    @Environment(\.tokens) private var tokens

    var body: some View {
        VStack(alignment: .leading, spacing: Space._2) {
            HStack(alignment: .top, spacing: Space._6) {
                column(
                    Icon.check.rawValue, tokens.statusSuccess, Words.text("settings-agents-with-status-hooks", fallback: "With status hooks"),
                    Words.text(
                        "settings-agents-the-agent-tells-wormhole-the-moment",
                        fallback: "The agent tells Wormhole the moment it starts working, waits for you or finishes."))
                column(
                    Icon.openTerminal.rawValue, tokens.textMuted, Words.text("settings-agents-without", fallback: "Without"),
                    Words.text(
                        "settings-agents-status-is-read-from-the-terminal",
                        fallback: "Status is read from the terminal output: usually right, sometimes a beat late."))
            }
            Text(
                Words.text(
                    "settings-agents-hooks-do-nothing-outside-wormhole-only",
                    fallback: "Hooks do nothing outside Wormhole, only add entries of their own, and a backup of every changed file is kept.")
            )
            .textRole(.caption).foregroundStyle(tokens.textMuted)
        }
        .padding(.vertical, Space._3)
    }

    private func column(_ icon: String, _ color: Color, _ title: String, _ text: String) -> some View {
        HStack(alignment: .top, spacing: Space._2) {
            Image(systemName: icon).font(.system(size: Metric.glyph)).foregroundStyle(color).frame(width: Metric.lead, height: Metric.lead)
            VStack(alignment: .leading, spacing: 0) {
                Text(title).textRole(.body, weight: .medium).foregroundStyle(tokens.textPrimary)
                Text(text).textRole(.caption).foregroundStyle(tokens.textMuted)
            }
            .frame(maxWidth: .infinity, alignment: .leading)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
    }
}
