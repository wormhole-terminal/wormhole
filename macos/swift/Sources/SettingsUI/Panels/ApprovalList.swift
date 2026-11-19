// The programs the person let drive Wormhole through its command line, MCP or socket (`sections/agent_access.rs`): a row
// each with how long it is allowed, who signed it and what it may do, and the one action that fits, Revoke. Nothing is
// allowed from here: an answer to a question in Wormhole's window is the only way a program gets in.

import DesignSystem
import SettingsModel
import SwiftUI
import WordsModel

struct ApprovalList: View {
    let records: [ApprovalRecord]
    let host: any SettingsHost
    @Environment(\.tokens) private var tokens

    var body: some View {
        VStack(spacing: 0) {
            if records.isEmpty {
                PanelNote(text: SettingsText.noApprovals)
            } else {
                ForEach(records) { approval in
                    HStack(spacing: Space._3) {
                        VStack(alignment: .leading, spacing: 0) {
                            Text(approval.label).textRole(.body, weight: .medium).foregroundStyle(tokens.textPrimary)
                            Text(approval.detail).textRole(.caption).foregroundStyle(tokens.textMuted)
                            if !approval.location.isEmpty {
                                Text(approval.location).textRole(.caption, design: .monospaced)
                                    .foregroundStyle(tokens.textMuted).lineLimit(1).truncationMode(.middle)
                            }
                            // What it may do, a tick each, changed at once and kept with the approval; closing always asks.
                            HStack(spacing: Space._3) {
                                ForEach(ApprovalCapability.allCases, id: \.self) { capability in
                                    SettingToggle(label: Self.word(capability), isOn: approval.capabilities.contains(capability), isCheckbox: true) {
                                        host.setCapability(capability, on: $0, of: approval)
                                    }
                                }
                            }
                            .padding(.top, Space._1)
                        }
                        Spacer(minLength: Space._3)
                        ActionButton(title: SettingsText.revoke, symbol: nil, frameId: "approval.\(approval.id)") { host.revoke(approval) }
                    }
                    .padding(.vertical, Space._2).frame(minHeight: SettingsMetric.rowMinHeight)
                    .accessibilityElement(children: .combine)
                    .accessibilityAction(named: SettingsText.revoke) { host.revoke(approval) }
                }
                PanelNote(text: SettingsText.approvalsNote)
            }
        }
    }

    private static func word(_ capability: ApprovalCapability) -> String {
        switch capability {
        case .read: Words.text("settings-agents-access-cap-read", fallback: "Read")
        case .type: Words.text("settings-agents-access-cap-type", fallback: "Type")
        case .run: Words.text("settings-agents-access-cap-run", fallback: "Run")
        case .open: Words.text("settings-agents-access-cap-open", fallback: "Open")
        }
    }
}
