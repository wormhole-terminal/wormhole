// Core's Extensions, Project folders, skill and flag rows as the UI's records. Pure functions over the generated records,
// so a test feeds canned ones; the words are already in the language in use.

import SettingsModel
import WormholeBindings

extension RecordMapping {
    static func probeList(_ list: ProbeListRecord) -> ProbeList {
        ProbeList(
            rows: list.rows.map(probe), keyProblem: list.keyProblem, tooMany: list.tooMany, folder: list.folder,
            folderPath: list.folderPath)
    }

    static func probe(_ row: ProbeRowRecord) -> ProbeRow {
        ProbeRow(
            id: row.id, title: row.title, identity: row.identity, description: row.description,
            state: ProbeRow.State(rawValue: row.state) ?? .off, stateWord: row.stateWord, unsealed: row.unsealed,
            note: row.note, canApprove: row.canApprove, canForget: row.canForget)
    }

    static func probeSheet(_ sheet: ProbeSheetRecord) -> ProbeSheet {
        ProbeSheet(
            id: sheet.id, title: sheet.title, identity: sheet.identity, description: sheet.description,
            since: sheet.since.map { ProbeSheet.Change(text: $0.text, added: $0.added) }, asksHeading: sheet.asksHeading,
            permissions: sheet.permissions.map {
                ProbePermission(
                    id: Int($0.index), words: $0.words, touchID: $0.touchId, granted: $0.granted, programNote: $0.programNote,
                    programWarns: $0.programWarns, commands: $0.commands)
            },
            combinations: sheet.combinations, filesFingerprint: sheet.filesFingerprint)
    }

    static func approval(_ outcome: ProbeApprovalOutcome) -> ProbeApproval {
        switch outcome {
        case .approved: .approved
        case .cancelled: .cancelled
        case .failed(let message): .failed(message)
        }
    }

    static func projectChoice(_ row: ProjectChoiceRecord) -> ProjectChoice {
        ProjectChoice(root: row.root, name: row.name, detail: row.detail)
    }

    static func projectDefault(_ row: ProjectDefaultRecord) -> ProjectDefault {
        ProjectDefault(key: row.key, label: row.label, description: row.description, on: row.on)
    }

    static func projectPage(_ page: ProjectPageRecord) -> ProjectPage {
        let trust: ProjectPage.Trust =
            switch page.status {
            case .trusted: .trusted
            case .changed: .changed
            case .untrusted: .untrusted
            }
        return ProjectPage(
            root: page.root, trust: trust, statusWord: page.statusWord, detail: page.detail,
            commands: page.commands.map { [$0.what, $0.runs, $0.note].filter { !$0.isEmpty }.joined(separator: " · ") },
            features: page.features.map {
                ProjectFeatureRow(
                    key: $0.key, label: $0.label, description: $0.description, faint: $0.faint, choice: $0.choice,
                    defaultOn: $0.defaultOn)
            },
            problems: page.problems, error: page.error, approving: page.approving, dirName: page.dirName)
    }

    static func skill(_ row: SettingsSkillRow) -> SkillRecord {
        let state: SkillRecord.State =
            switch row.state {
            case .notInstalled: .notInstalled
            case .current: .current
            case .outdated: .outdated
            case .foreign: .foreign
            case .covered: .covered
            case .unreadable: .unreadable
            }
        return SkillRecord(id: row.id, name: row.name, state: state, detail: row.detail, shown: row.shown, problem: row.problem)
    }

    static func flag(_ row: SettingsFlagRow) -> FlagRecord {
        FlagRecord(key: row.key, title: row.title, description: row.description, on: row.on, defaultOn: row.default)
    }
}
