// One setting (`style::setting`): the label (14 / 500) at the left with, after it, the (i) that opens the
// description in a card — or, for a warning that must not be missed, the description under the label — and
// the control at the right edge, at least `rowMinHeight` tall. A changed row has a dot before the label and a
// reset arrow (under the pointer, or always on the Changed page). The row has no hover fill of its own: the
// control answers the pointer. VoiceOver reads the row as a group with the label and description.

import DesignSystem
import SwiftUI

struct SettingRow<Control: View>: View {
    let label: String
    let description: String
    let visibleHint: Bool
    let state: RowState
    var tall = false
    let onReset: () -> Void
    @ViewBuilder let control: Control
    @Environment(\.tokens) private var tokens
    @State private var hovering = false

    var body: some View {
        HStack(alignment: tall ? .top : .center, spacing: Space._4) {
            VStack(alignment: .leading, spacing: 0) {
                HStack(spacing: Space._2) {
                    if state.isChanged { ChangedDot() }
                    MarkedText(text: label, range: state.found, font: .system(size: TextRole.body.size, weight: .medium))
                        .foregroundStyle(tokens.textPrimary)
                    if let context = state.context, !context.isEmpty {
                        Text(context).font(.system(size: TextRole.caption.size)).foregroundStyle(tokens.textFaint)
                    }
                    if !description.isEmpty, !visibleHint { InfoButton(text: description) }
                    if let key = state.configKey, hovering || state.configKey != nil {
                        Text(key).font(.system(size: TextRole.caption.size, design: .monospaced))
                            .foregroundStyle(tokens.textFaint).lineLimit(1)
                    }
                }
                .frame(minHeight: tall ? Metric.control : TextRole.body.size * 1.25 + 4)
                if visibleHint, !description.isEmpty {
                    MarkedText(text: description, range: state.foundDescription, font: .system(size: TextRole.caption.size))
                        .foregroundStyle(tokens.textMuted)
                }
            }
            .frame(maxWidth: .infinity, alignment: .leading)
            HStack(spacing: Space._2) {
                if state.isChanged, !state.resetAlways {
                    ResetArrow(action: onReset).opacity(hovering ? 1 : 0)
                }
                control
                if state.resetAlways {
                    Group { if state.isChanged { ResetArrow(action: onReset) } }.frame(width: Metric.hitMin)
                }
            }
        }
        .padding(.vertical, Space._1)
        .frame(minHeight: SettingsMetric.rowMinHeight)
        .background(state.isRevealed ? tokens.findMatch.opacity(0.25) : .clear, in: RoundedRectangle(cornerRadius: Radius.row))
        .motion(Motion.layout, value: state.isRevealed)
        .onHover { hovering = $0 }
        .accessibilityElement(children: .contain)
        .accessibilityLabel(label)
        .accessibilityHint(description)
    }
}
