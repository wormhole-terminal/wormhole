// One Probe: its mark, name and state, who and which version, what it says it is, the buttons that fit (Forget, Approve
// again) and the switch; under it what is wrong with it. An unsealed approval reads as off in a warning's colour.

import DesignSystem
import SettingsModel
import SwiftUI

struct ProbeRowView: View {
    let row: ProbeRow
    let probes: ProbesModel
    @Environment(\.tokens) private var tokens

    var body: some View {
        VStack(alignment: .leading, spacing: Space._2) {
            HStack(spacing: Space._3) {
                Image(systemName: "puzzlepiece.extension").font(.system(size: Metric.glyph)).foregroundStyle(tokens.textPrimary)
                    .frame(width: Metric.control, height: Metric.control)
                    .background(tokens.stateHover, in: RoundedRectangle(cornerRadius: tokens.metrics.radius))
                    .accessibilityHidden(true)
                VStack(alignment: .leading, spacing: 0) {
                    HStack(spacing: Space._2) {
                        Text(row.title).font(.system(size: TextRole.body.size, weight: .medium)).foregroundStyle(tokens.textPrimary).lineLimit(1)
                        StatusBadge(text: row.stateWord, color: color)
                    }
                    if !row.identity.isEmpty { caption(row.identity).lineLimit(1) }
                    if !row.description.isEmpty { caption(row.description) }
                }
                .frame(maxWidth: .infinity, alignment: .leading)
                if row.canForget { ActionButton(title: ExtensionsText.forget, symbol: nil) { Task { await probes.turnOff(row.id) } } }
                if row.state == .changed {
                    ActionButton(title: ExtensionsText.approveAgain, symbol: nil) { Task { await probes.openSheet(row.id) } }
                }
                FlatSwitch(isOn: row.state == .on, label: row.title) { on in Task { await probes.switched(row, on: on) } }
                    .disabled(!(row.canApprove || row.state == .on))
            }
            if let note = row.note {
                if row.state == .invalid {
                    Banner(kind: .danger, text: note).padding(.leading, Metric.control + Space._3)
                } else {
                    caption(note).padding(.leading, Metric.control + Space._3)
                }
            }
        }
        .padding(.vertical, Space._2)
        .accessibilityElement(children: .contain)
        .accessibilityLabel(row.title)
    }

    private var color: Color {
        if row.unsealed { return tokens.statusWarningText }
        switch row.state {
        case .on: return tokens.statusSuccessText
        case .changed: return tokens.statusWarningText
        case .invalid: return tokens.statusDangerText
        case .off, .unsupported: return tokens.textMuted
        }
    }

    private func caption(_ text: String) -> some View {
        Text(text).font(.system(size: TextRole.caption.size)).foregroundStyle(tokens.textMuted)
            .frame(maxWidth: .infinity, alignment: .leading).fixedSize(horizontal: false, vertical: true)
    }
}
