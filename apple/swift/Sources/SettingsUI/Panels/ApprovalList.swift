// The programs the person let drive Wormhole through its command line, MCP or socket (`sections/agent_access.rs`): a row
// each with how long it is allowed, who signed it and what it may do, and the one action that fits, Revoke. Nothing is
// allowed from here: an answer to a question in Wormhole's window is the only way a program gets in.

import DesignSystem
import SettingsModel
import SwiftUI

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
                            Text(approval.label).font(.system(size: TextRole.body.size, weight: .medium)).foregroundStyle(tokens.textPrimary)
                            Text(approval.detail).font(.system(size: TextRole.caption.size)).foregroundStyle(tokens.textMuted)
                            if !approval.location.isEmpty {
                                Text(approval.location).font(.system(size: TextRole.caption.size, design: .monospaced))
                                    .foregroundStyle(tokens.textMuted).lineLimit(1).truncationMode(.middle)
                            }
                            if !approval.allowed.isEmpty {
                                Text(approval.allowed.joined(separator: " · ")).font(.system(size: TextRole.caption.size))
                                    .foregroundStyle(tokens.textFaint)
                            }
                        }
                        Spacer(minLength: Space._3)
                        ActionButton(title: SettingsText.revoke, symbol: nil) { host.revoke(approval) }
                    }
                    .padding(.vertical, Space._2).frame(minHeight: SettingsMetric.rowMinHeight)
                    .accessibilityElement(children: .combine)
                    .accessibilityAction(named: SettingsText.revoke) { host.revoke(approval) }
                }
                PanelNote(text: SettingsText.approvalsNote)
            }
        }
    }
}
