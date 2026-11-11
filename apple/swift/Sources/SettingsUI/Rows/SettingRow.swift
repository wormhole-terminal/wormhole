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
    /// The row's id for `RowFrames` (nil: not recorded).
    var frameId: String?
    /// The row's config keys and its "Copy as JSON" (the row's menu, and a click on the key shown by Show config keys).
    var keys: [String] = []
    var json: () -> String = { "{}" }
    let onReset: () -> Void
    @ViewBuilder let control: Control
    @Environment(\.tokens) private var tokens
    @Environment(\.textScale) private var textScale
    @State private var hovering = false
    @State private var copiedKey = false

    var body: some View {
        HStack(alignment: tall ? .top : .center, spacing: Space._4) {
            VStack(alignment: .leading, spacing: 0) {
                HStack(spacing: Space._2) {
                    if state.isChanged { ChangedDot() }
                    MarkedText(text: label, range: state.found, role: .body, weight: .medium)
                        .foregroundStyle(tokens.textPrimary)
                    if let context = state.context, !context.isEmpty {
                        Text(context).textRole(.caption).foregroundStyle(tokens.textFaint)
                    }
                    if !description.isEmpty, !visibleHint { InfoButton(text: description) }
                    if let key = state.configKey { keyButton(key) }
                }
                .frame(minHeight: tall ? Metric.control : TextRole.body.size * textScale * 1.25 + 4)
                if visibleHint, !description.isEmpty {
                    MarkedText(text: description, range: state.foundDescription, role: .caption)
                        .foregroundStyle(tokens.textMuted)
                }
            }
            .frame(maxWidth: .infinity, alignment: .leading)
            HStack(spacing: Space._2) {
                if state.isChanged, !state.resetAlways {
                    ResetArrow(action: onReset).opacity(hovering ? 1 : 0).recordFrame(frameId.map { "\($0).reset" })
                }
                control.recordFrame(frameId)
                if state.resetAlways {
                    Group { if state.isChanged { ResetArrow(action: onReset) } }.frame(width: Metric.hitMin)
                }
            }
        }
        .padding(.vertical, Space._1)
        .frame(minHeight: SettingsMetric.rowMinHeight)
        .background(state.isRevealed ? tokens.findMatch.opacity(0.25) : .clear, in: RoundedRectangle(cornerRadius: Radius.row))
        .motion(Motion.layout, value: state.isRevealed)
        .contentShape(Rectangle())
        .onHover { hovering = $0 }
        .contextMenu {
            if !keys.isEmpty {
                Button(SettingsText.copyKey) { Pasteboard.setText(keys.joined(separator: "\n")) }
                Button(SettingsText.copyJSON) { Pasteboard.setText(json()) }
                Divider()
                Button(SettingsText.resetToDefault, action: onReset).disabled(!state.isChanged)
            }
        }
        .accessibilityElement(children: .contain)
        .accessibilityLabel(label)
        .accessibilityHint(description)
    }

    /// The key under Show config keys: a click copies the keys and says so for a moment.
    private func keyButton(_ key: String) -> some View {
        Button {
            Pasteboard.setText(keys.joined(separator: "\n"))
            Announcements.post(SettingsText.copied)
            copiedKey = true
        } label: {
            Text(copiedKey ? SettingsText.copied : key).textRole(.caption, design: .monospaced)
                .foregroundStyle(copiedKey ? tokens.statusSuccessText : tokens.textFaint).lineLimit(1)
        }
        .buttonStyle(.plain)
        .accessibilityLabel(SettingsText.copyKey)
        .accessibilityValue(key)
        .task(id: copiedKey) {
            guard copiedKey else { return }
            // one-shot: "Copied" goes back to the key
            try? await Task.sleep(for: .seconds(1.5))
            copiedKey = false
        }
    }
}
