// One setting (`style::setting`): the label (14 / 500) at the left with, after it, the (i) that opens the
// description in a card — or, for a warning that must not be missed, the description under the label — and
// the control at the right edge, at least `rowMinHeight` tall. A changed row has a dot before the label and a
// reset arrow (under the pointer, or always on the Changed page). The row has no hover fill of its own: the
// control answers the pointer. VoiceOver reads the row as a group with the label and description.
// A row with config keys has a menu (`style::row_keys`): Copy Key, Copy as JSON, Reset to Default, opened by a right click
// or by ⌃↩ on any control of the row, since a pointer's menu reaches neither the keyboard nor VoiceOver. A row without keys
// has no menu attached at all, so a text field in it keeps its own Cut, Copy and Paste (`own_menu`): the Profile page's
// fields, the only text fields in setting rows, sit in rows without keys. A disabled row is faint, its control held back
// and off, with no key and no menu (`SettingState::Disabled`).

import AppKit
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
    /// A tag after the label ("Time Sensitive", "Beta") and what sits under the row (a warning, a note, colour picks).
    var chip: String?
    var below: AnyView?
    let onReset: () -> Void
    @ViewBuilder let control: Control
    @Environment(\.tokens) private var tokens
    @Environment(\.textScale) private var textScale
    @State private var hovering = false
    @State private var copiedKey = false
    @Environment(\.optionHeld) private var optionHeld

    var body: some View {
        withMenu(row)
            // ⌃↩ on a control of the row: the same menu, where the pointer is, as `row_keys.rs` opens it.
            .onKeyPress(.return, phases: .down) { press in
                guard hasMenu, press.modifiers.subtracting(.capsLock) == .control else { return .ignored }
                showMenu()
                return .handled
            }
            .accessibilityElement(children: .contain)
            .accessibilityLabel(label)
            .accessibilityHint(description)
    }

    private var hasMenu: Bool { !keys.isEmpty && !state.isDisabled }

    /// The row's menu on the row, only where it has one: a row without is left as it is, so nothing of it covers a field.
    @ViewBuilder private func withMenu(_ row: some View) -> some View {
        if hasMenu {
            row.contextMenu {
                Button(SettingsText.copyKey) { copy(keys.joined(separator: "\n")) }
                Button(SettingsText.copyJSON) { copy(json()) }
                Divider()
                Button(SettingsText.resetToDefault, action: onReset).disabled(!state.isChanged)
            }
        } else {
            row
        }
    }

    private var row: some View {
        HStack(alignment: tall ? .top : .center, spacing: Space._4) {
            VStack(alignment: .leading, spacing: 0) {
                HStack(spacing: Space._2) {
                    if state.isChanged { ChangedDot() }
                    MarkedText(text: label, range: state.found, role: .body, weight: .medium)
                        .foregroundStyle(state.isDisabled ? tokens.textFaint : tokens.textPrimary)
                    if let context = state.context, !context.isEmpty {
                        Text(context).textRole(.caption).foregroundStyle(tokens.textFaint)
                    }
                    if let chip { RowChip(text: chip) }
                    if !description.isEmpty, !visibleHint { InfoButton(text: description) }
                    // Hidden, not removed: the row does not move when the key shows, and VoiceOver keeps its button.
                    if let key = state.configKey, !state.isDisabled { keyButton(key).opacity(hovering || optionHeld || copiedKey ? 1 : 0) }
                }
                .frame(minHeight: tall ? Metric.control : TextRole.body.size * textScale * 1.25 + 4)
                if visibleHint, !description.isEmpty {
                    MarkedText(text: description, range: state.foundDescription, role: .caption)
                        .foregroundStyle(state.isDisabled ? tokens.textFaint : tokens.textMuted)
                }
                if let below { below }
            }
            .frame(maxWidth: .infinity, alignment: .leading)
            HStack(spacing: Space._2) {
                if state.isChanged, !state.resetAlways {
                    ResetArrow(action: onReset).opacity(hovering ? 1 : 0).recordFrame(frameId.map { "\($0).reset" })
                }
                // Out of reach while disabled: held back, and off for the pointer, the keyboard and VoiceOver.
                control.opacity(state.isDisabled ? SettingsMetric.outOfReachOpacity : 1).disabled(state.isDisabled).recordFrame(frameId)
                if state.resetAlways {
                    Group { if state.isChanged { ResetArrow(action: onReset) } }.frame(width: Metric.hitMin)
                }
            }
        }
        .padding(.vertical, Space._1)
        .frame(minHeight: SettingsMetric.rowMinHeight)
        .background(state.isRevealed ? tokens.findMatch.opacity(0.25) : .clear, in: RoundedRectangle(cornerRadius: Radius.row))
        .motion(Motion.layout, value: state.isRevealed)
        .marked("setting.row")
        .contentShape(Rectangle())
        .onHover { hovering = $0 }
    }

    /// Puts `words` on the clipboard and says so: for a moment in the key's place, and to VoiceOver (`row_keys.rs` `copy`).
    private func copy(_ words: String) {
        Pasteboard.setText(words)
        Announcements.post(SettingsText.copied)
        copiedKey = true
    }

    /// The row's menu for ⌃↩. A test instance shows none: AppKit's menu loop would hold it.
    private func showMenu() {
        guard !TestInstance.isBackground else { return }
        let menu = NSMenu()
        menu.autoenablesItems = false
        menu.addItem(ActionMenuItem(SettingsText.copyKey) { copy(keys.joined(separator: "\n")) })
        menu.addItem(ActionMenuItem(SettingsText.copyJSON) { copy(json()) })
        menu.addItem(.separator())
        menu.addItem(ActionMenuItem(SettingsText.resetToDefault, enabled: state.isChanged, run: onReset))
        menu.popUp(positioning: nil, at: NSEvent.mouseLocation, in: nil)
    }

    /// The key under Show config keys: a click copies the keys and says so for a moment.
    private func keyButton(_ key: String) -> some View {
        Button {
            copy(keys.joined(separator: "\n"))
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
