// One host as a card: a badge for how it stands, its id, the buttons, and the facts in two columns (`card.rs`).
// The card's edge turns to the warning colour when something needs attention.

import AuxModel
import DesignSystem
import SwiftUI

struct HostCardView: View {
    let card: HostCard
    let copied: Bool
    let copy: () -> Void
    let end: () -> Void
    let forget: () -> Void
    @Environment(\.tokens) private var tokens

    /// The label column's width (`LABEL_W`).
    private static let labelWidth: CGFloat = 104

    var body: some View {
        AuxCard(edge: card.needsAttention ? tokens.statusWarning : nil) {
            VStack(alignment: .leading, spacing: Space._2) {
                HStack(spacing: Space._3) {
                    StatusBadge(text: card.standing.label, color: color)
                    Text(card.id).font(TextRole.body.font.weight(.semibold))
                    Text(card.note).font(TextRole.label.font).foregroundStyle(tokens.textMuted).lineLimit(1)
                    Spacer(minLength: 0)
                    Button(copied ? AuxText.hostsCopied : AuxText.hostsCopy, action: copy).secondaryAction().controlSize(.small)
                    if card.canEnd {
                        Button(AuxText.hostsEnd, action: end).secondaryAction().controlSize(.small)
                    } else if card.canForget {
                        Button(AuxText.hostsForget, action: forget).secondaryAction().controlSize(.small)
                    }
                }
                Grid(alignment: .topLeading, horizontalSpacing: Space._2, verticalSpacing: Space._1) {
                    ForEach(rows, id: \.first?.id) { row in
                        GridRow {
                            ForEach(row) { fact in
                                label(fact)
                                value(fact).gridCellColumns(row.count == 1 ? 3 : 1)
                            }
                        }
                    }
                }
            }
        }
        .accessibilityElement(children: .combine)
        .accessibilityLabel(card.text)
    }

    /// Wide facts take a row to themselves; the rest sit two to a row.
    private var rows: [[HostFact]] {
        var rows: [[HostFact]] = [], pending: [HostFact] = []
        for fact in card.facts {
            if fact.field.wide {
                if !pending.isEmpty { rows.append(pending); pending = [] }
                rows.append([fact])
            } else {
                pending.append(fact)
                if pending.count == 2 { rows.append(pending); pending = [] }
            }
        }
        return pending.isEmpty ? rows : rows + [pending]
    }

    private var color: Color {
        switch card.standing {
        case .attached: tokens.statusSuccessText
        case .detached, .orphan: tokens.statusWarningText
        case .stale: tokens.textMuted
        }
    }

    private func label(_ fact: HostFact) -> some View {
        Text(fact.field.label).font(TextRole.label.font.weight(.regular)).foregroundStyle(tokens.textMuted)
            .frame(width: Self.labelWidth, alignment: .leading)
    }

    private func value(_ fact: HostFact) -> some View {
        Text(fact.value).font(TextRole.label.font.weight(.regular)).lineLimit(1)
            .foregroundStyle(tone(fact.tone)).textSelection(.enabled)
            .frame(maxWidth: .infinity, alignment: .leading)
    }

    private func tone(_ tone: HostFact.Tone) -> Color {
        switch tone {
        case .normal: tokens.textPrimary
        case .muted: tokens.textMuted
        case .warning: tokens.statusWarningText
        case .danger: tokens.statusDangerText
        }
    }
}
