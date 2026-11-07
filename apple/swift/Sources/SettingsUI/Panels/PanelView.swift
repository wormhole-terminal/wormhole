// The blocks and records of the pages that are lists rather than switches. A block (the agents, what can be
// imported) draws the host's records on the card; a row panel (a status and a button: updates, notification
// permission, the profile's fields) sits in a row with its label. Anything the build can't do yet says so
// instead of drawing a dead control.

import DesignSystem
import SettingsModel
import SwiftUI

struct PanelView: View {
    let model: SettingsModel
    let entry: SettingsSchema.Entry
    let name: String

    var body: some View {
        let records = model.host.panelRecords()
        switch name {
        case "agentsInstalled": AgentList(records: records.agents.filter { $0.state != .notInstalled }, host: model.host)
        case "agentsOther": AgentList(records: records.agents.filter { $0.state == .notInstalled }, host: model.host)
        case "importFound": ImportList(records: records.imports)
        case "agentAccessPrograms": ApprovalList(records: records.approvals, host: model.host)
        case "agentsHow": HowStatusWorks()
        case "agent-skills": SkillList(records: records.skills, host: model.host)
        case "feature-flags": FlagList(model: model)
        case "probes": ProbesPanel(model: model)
        case "projects": ProjectsPanel(model: model)
        default: PanelNote(text: entry.description.isEmpty ? entry.label : entry.description)
        }
    }
}

extension ControlSpec {
    /// Panels that are one line of status at the right of a labelled row, not a block of records.
    var isRowPanel: Bool {
        guard case .panel(let name) = self else { return false }
        return !["agentsInstalled", "agentsOther", "agentsHow", "agent-skills", "importFound", "count-details", "projects", "probes", "feature-flags", "agentAccessPrograms"].contains(name)
    }
}

struct PanelNote: View {
    let text: String
    @Environment(\.tokens) private var tokens
    var body: some View {
        Text(text).font(.system(size: TextRole.caption.size)).foregroundStyle(tokens.textMuted)
            .frame(maxWidth: .infinity, alignment: .leading).padding(.vertical, Space._3)
    }
}
