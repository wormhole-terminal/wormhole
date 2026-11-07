// The approval sheet (`sections/extensions_sheet.rs`): what a Probe asks for, in plain words, each permission with a
// switch of its own, and the one confirmation that fits (a click, or Touch ID when anything left on is that level or the
// Probe makes a dangerous combination). It approves exactly what it showed: the core read the Probe once for it. Cancel,
// a cancelled Touch ID and a write that fails change nothing; nothing on it changes while the system asks.

import DesignSystem
import SettingsModel
import SwiftUI

struct ProbeSheetView: View {
    let probes: ProbesModel
    @Environment(\.tokens) private var tokens

    /// The sheet's width (`SHEET_W`).
    private static let width: CGFloat = 520

    var body: some View {
        if let open = probes.open {
            let sheet = open.sheet
            VStack(alignment: .leading, spacing: Space._3) {
                ScrollView {
                    VStack(alignment: .leading, spacing: Space._3) {
                        Text(sheet.title).font(TextRole.heading.font).foregroundStyle(tokens.textPrimary)
                        caption(sheet.identity)
                        Text(sheet.description).font(.system(size: TextRole.body.size)).foregroundStyle(tokens.textPrimary)
                        if !sheet.since.isEmpty { since(sheet.since) }
                        SectionLabel(sheet.asksHeading)
                        VStack(spacing: 0) {
                            ForEach(sheet.permissions) { permission in
                                RowDivider()
                                PermissionRow(permission: permission, open: open, probes: probes)
                            }
                        }
                        if !sheet.combinations.isEmpty {
                            SectionLabel(ExtensionsText.combinations)
                            ForEach(sheet.combinations, id: \.self) { Banner(kind: .danger, text: $0) }
                        }
                    }
                    .padding(.bottom, Space._2)
                }
                .scrollBounceBehavior(.basedOnSize)
                caption(sheet.filesFingerprint)
                caption(open.needsTouchID ? ExtensionsText.confirmTouchID : ExtensionsText.confirmClick)
                if let error = open.error {
                    Text(error).font(.system(size: TextRole.caption.size)).foregroundStyle(tokens.statusDangerText)
                        .fixedSize(horizontal: false, vertical: true)
                }
                HStack(spacing: Space._2) {
                    Spacer(minLength: 0)
                    ActionButton(title: ExtensionsText.cancel, symbol: nil) { probes.close() }
                        .keyboardShortcut(.cancelAction).disabled(open.busy)
                    ActionButton(title: label(open), symbol: nil) { Task { await probes.approve() } }
                        .keyboardShortcut(.defaultAction).disabled(open.busy)
                }
            }
            .padding(Space._4).frame(width: Self.width).frame(maxHeight: 560).background(tokens.surfaceOverlay)
            .accessibilityElement(children: .contain)
            .accessibilityLabel(sheet.title)
        }
    }

    private func label(_ open: ProbesModel.Open) -> String {
        open.busy ? ExtensionsText.waiting : open.needsTouchID ? ExtensionsText.approveTouchID : ExtensionsText.approve
    }

    private func since(_ changes: [ProbeSheet.Change]) -> some View {
        VStack(alignment: .leading, spacing: Space._1) {
            SectionLabel(ExtensionsText.since)
            ForEach(Array(changes.enumerated()), id: \.offset) { _, change in
                Text(change.text).font(.system(size: TextRole.body.size))
                    .foregroundStyle(change.added ? tokens.statusWarningText : tokens.textPrimary)
            }
        }
    }

    private func caption(_ text: String) -> some View {
        Text(text).font(.system(size: TextRole.caption.size)).foregroundStyle(tokens.textMuted).fixedSize(horizontal: false, vertical: true)
    }
}

private struct PermissionRow: View {
    let permission: ProbePermission
    let open: ProbesModel.Open
    let probes: ProbesModel
    @Environment(\.tokens) private var tokens

    var body: some View {
        VStack(alignment: .leading, spacing: Space._1) {
            HStack(spacing: Space._2) {
                Text(permission.words).font(.system(size: TextRole.body.size)).foregroundStyle(tokens.textPrimary)
                    .frame(maxWidth: .infinity, alignment: .leading).fixedSize(horizontal: false, vertical: true)
                if permission.touchID { StatusBadge(text: ExtensionsText.touchID, color: tokens.textMuted) }
                SettingToggle(label: permission.words, isOn: open.granted.contains(permission.id)) { _ in
                    Task { await probes.toggle(permission) }
                }
                .disabled(open.busy)
            }
            if let note = permission.programNote {
                Text(note).font(.system(size: TextRole.caption.size))
                    .foregroundStyle(permission.programWarns ? tokens.statusDangerText : tokens.textMuted)
            }
            ForEach(permission.commands, id: \.self) { command in
                Text(command).font(.system(size: TextRole.caption.size, design: .monospaced)).foregroundStyle(tokens.textPrimary)
                    .padding(.horizontal, Space._1)
                    .background(tokens.stateHover, in: RoundedRectangle(cornerRadius: tokens.metrics.radiusChip))
            }
        }
        .padding(.vertical, Space._2)
    }
}
