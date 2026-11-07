// Core's rows for the host's pages as the UI's records. Pure functions over the generated records, so a test feeds
// canned ones; the words (details, summaries) are already in the catalogs' language.

import SettingsModel
import WormholeBindings

enum RecordMapping {
    static func agent(_ row: SettingsAgentRow) -> AgentRecord {
        let make = { (state: AgentRecord.State, hooks: AgentRecord.HookAction?, detail: String) in
            AgentRecord(id: row.id, name: row.name, detail: detail, state: state, hooks: hooks)
        }
        guard row.onPath else { return make(.notInstalled, nil, row.detail) }
        switch row.hooks {
        case .installed: return make(.ready, .remove, row.detail)
        case .partial: return make(.needsRepair, .repair, row.detail)
        case .notInstalled: return make(.statusFromOutput, .install, row.detail)
        case .noHooks: return make(.statusFromOutput, nil, row.detail)
        case .unreadable(let reason): return make(.needsRepair, nil, "\(row.detail) · \(reason)")
        }
    }

    static func imported(_ row: SettingsImportRow) -> ImportRecord {
        ImportRecord(id: row.id, app: row.app, summary: row.summary)
    }

    /// The profile Wormhole is in; Default's own start folder is the home folder, shown as `~`.
    static func profile(_ rows: [SettingsProfileRow]) -> ProfileRecord? {
        guard let row = rows.first(where: \.isActive) ?? rows.first else { return nil }
        return ProfileRecord(
            name: row.name, startFolder: row.startDir.isEmpty ? "~" : row.startDir, gitName: row.gitName,
            gitEmail: row.gitEmail)
    }

    static func option(_ row: SettingsOptionRow) -> SettingOption {
        SettingOption(value: .string(row.value), label: row.label)
    }
}

// The Command line tool, the approvals, the profiles and the project in front.
extension RecordMapping {
    static func commandLine(_ row: CliToolRecord) -> CommandLineToolRecord {
        let state: CommandLineToolRecord.State =
            switch row.state {
            case .notInstalled: .notInstalled
            case .installed: .installed
            case .foreign: .foreign
            }
        return CommandLineToolRecord(
            state: state, command: row.command, headline: row.headline, note: row.note, current: row.current,
            folder: row.folder)
    }

    static func pathNote(_ row: CliToolPathRecord) -> CommandLinePathNote {
        let verdict: CommandLinePathNote.Verdict =
            switch row.verdict {
            case .on: .on
            case .off: .off
            case .unknown: .unknown
            }
        return CommandLinePathNote(verdict: verdict, text: row.text, line: row.adviceLine)
    }

    static func approval(_ row: AccessApprovalRecord) -> ApprovalRecord {
        ApprovalRecord(
            number: row.id, isStored: row.stored, label: row.label, detail: row.detail, location: row.location,
            allowed: row.allowed, agent: row.agent)
    }

    static func choice(_ row: SettingsProfileRow) -> ProfileChoice {
        ProfileChoice(
            id: row.id, name: row.name, isDefault: row.isDefault, isActive: row.isActive,
            deleteQuestion: profileDeleteQuestion(profile: row.name), copyLabel: profileCopyOption(profile: row.name))
    }

    /// The rows a project sets, marked: nil when it sets none (nothing to say on any page).
    static func project(_ row: ProjectSettingsRecord, entries: [String]) -> ProjectSettingsMark? {
        entries.isEmpty ? nil : ProjectSettingsMark(name: row.name, folder: row.dir, root: row.root, entries: Set(entries), note: row.note)
    }
}
