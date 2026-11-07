// "Ready when you are": no coding agent on PATH, so a primer instead: Wormhole is a terminal first, the keys worth
// knowing, and one switch (`render_primer_step`). The keys are the key table's glyphs.

import AuxModel
import DesignSystem
import SwiftUI

struct PrimerStep: View {
    let session: OnboardingSession
    @Environment(\.tokens) private var tokens

    /// What to try first and its keys (the menus' glyphs, from core's key table).
    static let things: [(String, [String])] = [
        ("Jump to anything", ["\u{21E7}\u{2318}P"]), ("See every shortcut", ["\u{2325}\u{2318}/"]),
        ("Show what changed in git", ["\u{2318}9"]), ("Hide the side panels", ["\u{2318}B", "\u{2325}\u{2318}B"]),
    ]

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            Rule()
            HStack(alignment: .top, spacing: Space._2) {
                Image(systemName: "terminal").frame(width: Metric.lead)
                VStack(alignment: .leading, spacing: 0) {
                    Text(AuxText.useAsTerminal).font(TextRole.body.font.weight(.medium))
                    Text(AuxText.useAsTerminalDetail).font(TextRole.caption.font).foregroundStyle(tokens.listSelectedMuted)
                }
            }
            .foregroundStyle(tokens.listSelectedText)
            .padding(.horizontal, Space._4).padding(.vertical, Space._2)
            .frame(maxWidth: .infinity, alignment: .leading)
            .background(tokens.listSelected)
            SectionLabel(AuxText.thingsToTry).padding(.horizontal, Space._6).padding(.top, Space._4)
            ForEach(Array(Self.things.enumerated()), id: \.offset) { _, thing in
                Rule()
                HStack {
                    Text(thing.0).font(TextRole.body.font)
                    Spacer()
                    HStack(spacing: Space._1) { ForEach(thing.1, id: \.self) { KeyCap(text: $0) } }
                }
                .padding(.horizontal, Space._6).frame(minHeight: Metric.row + Space._1)
            }
            Rule()
            HStack(alignment: .top, spacing: Space._2) {
                Image(systemName: "info.circle")
                Text(AuxText.noAgentFound(others: session.othersFound)).fixedSize(horizontal: false, vertical: true)
            }
            .font(TextRole.caption.font).foregroundStyle(tokens.textMuted)
            .padding(.horizontal, Space._6).padding(.vertical, Space._3)
            Rule()
            NotifyToggle(label: AuxText.notifyCommand, isOn: Bindable(session.preferences).notifyCommandDone)
        }
    }
}
