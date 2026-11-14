// The agents Wormhole found and where their status hooks stand (`agents.rs`): a tile with the agent's mark, its name with a
// badge for the hooks, where they are (Reveal in Finder), the one button that fits, and what is left to do in the agent.
// "Other agents" is one folded row naming the ones that are not on this Mac's PATH.

import AppKit
import DesignSystem
import SettingsModel
import SwiftUI
import WordsModel

struct AgentList: View {
    let records: [AgentRecord]
    var logins: [AgentLoginRecord] = []
    /// False until the first read lands: the list says it is looking instead of claiming there is nothing.
    var read = true
    let host: any SettingsHost
    @Environment(\.tokens) private var tokens

    var body: some View {
        VStack(spacing: 0) {
            ForEach(Array(records.enumerated()), id: \.element.id) { index, agent in
                if index > 0 { Rectangle().fill(tokens.settingsDivider).frame(height: tokens.metrics.borderWidth) }
                row(agent)
            }
            if records.isEmpty, !read {
                PanelNote(text: Words.text("settings-agents-looking-for-agents", fallback: "Looking for agents…"))
            } else if records.isEmpty {
                PanelNote(text: Words.text("settings-agents-no-coding-agent-on-this-macs", fallback: "No coding agent on this Mac's PATH. Install Claude Code, Codex or one of the others below: Wormhole recognises it in any terminal, and offers its hooks here."))
            }
        }
    }

    private func row(_ agent: AgentRecord) -> some View {
        VStack(alignment: .leading, spacing: Space._2) {
            HStack(spacing: Space._3) {
                AgentMark(text: agent.mark, size: Metric.tile)
                VStack(alignment: .leading, spacing: 0) {
                    HStack(spacing: Space._2) {
                        Text(agent.name).textRole(.body, weight: .medium).foregroundStyle(tokens.textPrimary)
                        StatusBadge(text: badge(agent.state).text, color: badge(agent.state).color)
                    }
                    HStack(spacing: Space._1) {
                        Text(agent.detail).textRole(.caption).foregroundStyle(tokens.textMuted).lineLimit(1).truncationMode(.middle)
                        if let path = agent.configPath {
                            IconButton(symbol: Icon.reveal.rawValue, label: Words.text("settings-agents-reveal-in-finder", fallback: "Reveal in Finder")) {
                                ExternalOpen.reveal([URL(fileURLWithPath: path)])
                            }
                            .frame(width: Metric.controlSmall, height: Metric.controlSmall)
                        }
                    }
                }
                Spacer(minLength: 0)
                if let hooks = agent.hooks { ActionButton(title: title(hooks), symbol: nil, frameId: "agent.\(agent.id).hooks") { host.perform(hooks, agent: agent.id) } }
            }
            if let note = agent.note {
                Text(note).textRole(.caption).foregroundStyle(tokens.textMuted).padding(.leading, Metric.tile + Space._3)
            }
            // Each profile's own login of this agent: a second place for the hooks, with its own button.
            ForEach(logins.filter { $0.agent == agent.id }) { login in loginRow(login) }
        }
        .padding(.vertical, Space._2)
        .frame(minHeight: SettingsMetric.rowMinHeight)
        .accessibilityElement(children: .combine)
    }

    private func loginRow(_ login: AgentLoginRecord) -> some View {
        HStack(spacing: Space._2) {
            Text(login.line).textRole(.caption).foregroundStyle(tokens.textMuted)
            Text(login.shown).textRole(.caption, design: .monospaced).foregroundStyle(tokens.textMuted).lineLimit(1).truncationMode(.middle)
                .padding(.horizontal, Space._1)
                .background(tokens.controlBackground, in: RoundedRectangle(cornerRadius: tokens.metrics.radiusChip))
            Spacer(minLength: 0)
            if let hooks = login.hooks {
                ActionButton(title: title(hooks), symbol: nil, frameId: "agent.login.\(login.id)") { host.perform(hooks, login: login) }
            }
        }
        .padding(.leading, Metric.tile + Space._3)
        .accessibilityElement(children: .combine)
    }

    private func badge(_ state: AgentRecord.State) -> (text: String, color: Color) {
        switch state {
        case .ready: (Words.text("settings-agents-status-hooks-on", fallback: "Status hooks on"), tokens.statusSuccessText)
        case .needsRepair: (Words.text("settings-agents-hooks-incomplete", fallback: "Hooks incomplete"), tokens.statusWarningText)
        case .statusFromOutput, .notInstalled: (Words.text("settings-agents-reading-output", fallback: "Reading output"), tokens.textMuted)
        }
    }

    private func title(_ hooks: AgentRecord.HookAction) -> String {
        switch hooks {
        case .install: Words.text("settings-agents-install-hooks", fallback: "Install hooks")
        case .repair: Words.text("settings-agents-repair-hooks", fallback: "Repair hooks")
        case .remove: Words.text("settings-agents-remove-hooks", fallback: "Remove hooks")
        }
    }
}

/// An agent's tile: its letters on the tile colour (`style::status_tile_at` without a status), so it says who the agent is
/// and nothing about what it is doing.
struct AgentMark: View {
    let text: String
    let size: CGFloat
    @Environment(\.tokens) private var tokens

    var body: some View {
        RoundedRectangle(cornerRadius: size * 0.28).fill(tokens.tileBackground).frame(width: size, height: size)
            .overlay(Text(text).font(.system(size: size * 0.4, weight: .bold)).foregroundStyle(tokens.tileMark))
            .accessibilityHidden(true)
    }
}

/// The agents that are not on the PATH: a folded row ("17 more agents Wormhole recognises"), the names under it, and
/// each agent with its tile and whether hooks are on offer when it is open.
struct OtherAgents: View {
    let records: [AgentRecord]
    let host: any SettingsHost
    @State private var open = false
    @Environment(\.tokens) private var tokens

    var body: some View {
        if !records.isEmpty {
            VStack(alignment: .leading, spacing: 0) {
                Button { open.toggle() } label: {
                    HStack(alignment: .top, spacing: Space._2) {
                        Image(Icon.disclosure).font(.system(size: Metric.chevron, weight: .bold)).rotationEffect(.degrees(open ? 90 : 0))
                            .motionTurn(Motion.layout, value: open)
                            .frame(width: Metric.lead, height: Metric.control).foregroundStyle(tokens.textMuted)
                        VStack(alignment: .leading, spacing: 0) {
                            Text(host.othersHead(count: records.count)).textRole(.body, weight: .medium).foregroundStyle(tokens.textPrimary)
                            if !open {
                                Text(records.map(\.name).joined(separator: " · ")).textRole(.caption).foregroundStyle(tokens.textMuted)
                                    .multilineTextAlignment(.leading)
                            }
                        }
                        Spacer(minLength: Space._2)
                        Text(Words.text("settings-agents-not-on-path", fallback: "Not on PATH")).textRole(.caption).foregroundStyle(tokens.textFaint)
                            .frame(height: Metric.control)
                    }
                    .padding(.vertical, Space._1).contentShape(Rectangle())
                }
                .buttonStyle(.plain)
                .accessibilityValue(open ? "expanded" : "collapsed")
                if open {
                    ForEach(records) { agent in
                        HStack(spacing: Space._3) {
                            AgentMark(text: agent.mark, size: Metric.tileMini)
                            Text(agent.name).textRole(.body).foregroundStyle(tokens.textMuted)
                            Spacer(minLength: 0)
                            Text(agent.detail).textRole(.caption).foregroundStyle(tokens.textFaint)
                        }
                        .padding(.vertical, Space._1).padding(.leading, Metric.lead + Space._2)
                        .accessibilityElement(children: .combine)
                    }
                }
            }
            .padding(.bottom, open ? Space._2 : 0)
            .motion(Motion.layout, value: open)
        }
    }
}
