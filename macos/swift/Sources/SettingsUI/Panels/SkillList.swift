// The Wormhole skill of each agent that loads skills (`sections/agent_skills.rs`): the state as a dot and a line with the
// skill's folder, and the one action that fits (Install, Update and Uninstall, or nothing when another folder the agent
// reads has it). Only a row's own button writes: opening the page changes nothing.

import DesignSystem
import SettingsModel
import SwiftUI
import WordsModel

struct SkillList: View {
    let records: [SkillRecord]
    let host: any SettingsHost
    @Environment(\.tokens) private var tokens

    var body: some View {
        VStack(spacing: 0) {
            ForEach(records) { skill in
                VStack(alignment: .leading, spacing: Space._2) {
                    HStack(spacing: Space._3) {
                        Circle().fill(color(skill.state)).frame(width: Metric.statusDot, height: Metric.statusDot)
                        VStack(alignment: .leading, spacing: 0) {
                            Text(skill.name).textRole(.body, weight: .medium).foregroundStyle(tokens.textPrimary)
                            HStack(spacing: Space._1) {
                                Text(skill.detail)
                                Text(skill.shown).textRole(.caption, design: .monospaced)
                                    .padding(.horizontal, Space._1).lineLimit(1).truncationMode(.middle)
                                    .background(tokens.stateHover, in: RoundedRectangle(cornerRadius: tokens.metrics.radiusChip))
                            }
                            .textRole(.caption).foregroundStyle(color(skill.state))
                        }
                        Spacer(minLength: Space._3)
                        actions(skill)
                    }
                    if let problem = skill.problem { Banner(kind: .danger, text: problem).padding(.leading, Metric.control) }
                }
                .padding(.vertical, Space._2).frame(minHeight: SettingsMetric.rowMinHeight)
                .accessibilityElement(children: .combine)
            }
            if records.isEmpty {
                PanelNote(text: Words.text("settings-agents-skill-none", fallback: "No coding agent that loads skills was found on this Mac's PATH."))
            }
        }
    }

    @ViewBuilder private func actions(_ skill: SkillRecord) -> some View {
        HStack(spacing: Space._2) {
            switch skill.state {
            case .current: remove(skill)
            case .outdated:
                button(SettingsText.update, skill, install: true)
                remove(skill)
            case .foreign: ActionButton(title: SettingsText.install, symbol: nil) {}.disabled(true)
                    .help(Words.text("settings-agents-skill-foreign-tooltip", fallback: "A skill with this name is there that Wormhole didn't write; it never replaces it"))
            case .covered, .unreadable: EmptyView()
            case .notInstalled: button(SettingsText.install, skill, install: true)
            }
        }
    }

    private func button(_ title: String, _ skill: SkillRecord, install: Bool) -> some View {
        ActionButton(title: title, symbol: nil, frameId: "skill.\(skill.id).\(install ? "install" : "remove")") { host.perform(skill: skill.id, install: install) }  // text: a frame id for the driver
    }

    private func remove(_ skill: SkillRecord) -> some View { button(SettingsText.uninstall, skill, install: false) }

    private func color(_ state: SkillRecord.State) -> Color {
        switch state {
        case .current: tokens.statusSuccessText
        case .outdated, .foreign: tokens.statusWarningText
        case .notInstalled, .covered, .unreadable: tokens.textMuted
        }
    }
}

/// The `claude mcp add …` line and a button that copies it (`draw_mcp`): Wormhole edits no other app's MCP settings.
struct McpControl: View {
    let command: String
    @Environment(\.tokens) private var tokens

    var body: some View {
        HStack(spacing: Space._2) {
            Text(command).textRole(.caption, design: .monospaced).foregroundStyle(tokens.textPrimary)
                .fixedSize().padding(.horizontal, Space._2).frame(height: Metric.control)
                .background(tokens.stateHover, in: RoundedRectangle(cornerRadius: tokens.metrics.radius))
            CopyButton(title: SettingsText.copyCommand, text: command)
        }
    }
}
