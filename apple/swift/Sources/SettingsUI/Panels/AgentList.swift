// The agents Wormhole found and what state their status hooks are in (`agents.rs`): a dot for the state, the
// name and where it was found, and the button that sets it up or repairs it.

import DesignSystem
import SettingsModel
import SwiftUI

struct AgentList: View {
    let records: [AgentRecord]
    let host: any SettingsHost
    @Environment(\.tokens) private var tokens

    var body: some View {
        VStack(spacing: 0) {
            ForEach(records) { agent in
                HStack(spacing: Space._3) {
                    Circle().fill(color(agent.state)).frame(width: Metric.statusDot, height: Metric.statusDot)
                    VStack(alignment: .leading, spacing: 0) {
                        Text(agent.name).font(.system(size: TextRole.body.size, weight: .medium)).foregroundStyle(tokens.textPrimary)
                        Text(agent.detail).font(.system(size: TextRole.caption.size)).foregroundStyle(tokens.textMuted)
                    }
                    Spacer()
                    if let hooks = agent.hooks {
                        ActionButton(title: title(hooks), symbol: nil) { host.perform(hooks, agent: agent.id) }
                    }
                }
                .frame(minHeight: SettingsMetric.rowMinHeight)
                .accessibilityElement(children: .combine)
            }
            if records.isEmpty { PanelNote(text: String(localized: "settings-agents-none", defaultValue: "None found on this Mac")) }
        }
    }

    private func color(_ state: AgentRecord.State) -> Color {
        switch state {
        case .ready: tokens.statusSuccess
        case .needsRepair: tokens.statusWarning
        case .statusFromOutput: tokens.textMuted
        case .notInstalled: tokens.textFaint
        }
    }

    private func title(_ hooks: AgentRecord.HookAction) -> String {
        switch hooks {
        case .install: String(localized: "settings-agents-install-hooks", defaultValue: "Install hooks")
        case .repair: String(localized: "settings-agents-repair-hooks", defaultValue: "Repair hooks")
        case .remove: String(localized: "settings-agents-remove-hooks", defaultValue: "Remove hooks")
        }
    }
}
