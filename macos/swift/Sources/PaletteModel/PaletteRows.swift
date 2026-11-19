// The lines of the palette for a scope and a query (core's `command_palette/rows`): a group's label with its
// count, a result with the characters the search found, or a note where a list has nothing. Selection only
// ever lands on a result.

import Foundation
import WordsModel
import WorkspaceModel

public enum PaletteRow: Sendable, Equatable, Identifiable {
    case label(title: String, count: Int?)
    /// The label over the trusted project's own actions, "Project · terminal/.wormhole", with its Trusted badge.
    case projectLabel(String)
    case result(PaletteResult)
    case note(String)

    public var id: String {
        switch self {
        case .label(let title, _): "label-\(title)"
        case .projectLabel(let title): "project-label-\(title)"
        case .result(let result): result.id
        case .note(let text): "note-\(text)"
        }
    }

    public var result: PaletteResult? {
        if case .result(let result) = self { result } else { nil }
    }
}

public struct PaletteResult: Sendable, Equatable, Identifiable {
    public var command: Command
    /// Character offsets into `command.title` to mark.
    public var ranges: [Range<Int>]
    public var id: String { command.id }
}

public enum PaletteRows {
    /// `query` is the words after any prefix. `recent` is the ids chosen before, newest first; `suggested` the action ids
    /// listed with nothing typed (the core's, for this window).
    public static func rows(
        scope: PaletteScope, query: String, commands: [Command], recent: [String] = [], suggested: [String] = [],
        project: String? = nil
    ) -> [PaletteRow] {
        let query = query.trimmingCharacters(in: .whitespaces)
        let rows =
            switch (scope, query.isEmpty) {
            case (.all, true): suggestions(commands, recent: recent, suggested: suggested)
            case (.terminals, true): terminalsByState(commands)
            case (.actions, true): actionsBySection(commands, project: project)
            case (.settings, true): recentSettings(commands, recent: recent)
            default: found(scope: scope, query: query, commands: commands)
            }
        return rows.isEmpty ? [.note(query.isEmpty ? Words.text("palette-empty-nothing-yet", fallback: "Nothing here yet") : Words.text("palette-empty-nothing-matches", fallback: "Nothing matches"))] : rows
    }

    /// How many results a query found (the footer's count): every group's whole count.
    public static func resultCount(_ rows: [PaletteRow]) -> Int {
        rows.reduce(0) { total, row in
            if case .label(_, let count?) = row { total + count } else { total }
        }
    }

    private static func results(_ commands: [Command]) -> [PaletteRow] {
        commands.map { .result(PaletteResult(command: $0, ranges: [])) }
    }

    /// Nothing typed: what was chosen before, then the actions that fit most windows, each with its key.
    private static func suggestions(_ commands: [Command], recent: [String], suggested: [String]) -> [PaletteRow] {
        let lookup = Dictionary(commands.map { ($0.id, $0) }, uniquingKeysWith: { first, _ in first })
        let recents = recent.compactMap { lookup[$0] }.filter { $0.group != .actions }.prefix(6)
        var rows: [PaletteRow] = recents.isEmpty ? [] : [.label(title: Words.text("folder-picker-recent", fallback: "Recent"), count: nil)] + results(Array(recents))
        let actions = suggested.compactMap { lookup["action-\($0)"] }
        if !actions.isEmpty { rows += [.label(title: Words.text("palette-group-actions", fallback: "Actions"), count: nil)] + results(actions) }
        return rows
    }

    /// The Settings scope with nothing typed: the settings chosen lately; typing finds any setting.
    private static func recentSettings(_ commands: [Command], recent: [String]) -> [PaletteRow] {
        let lookup = Dictionary(commands.map { ($0.id, $0) }, uniquingKeysWith: { first, _ in first })
        let settings = recent.compactMap { lookup[$0] }.filter { $0.group == .settings }
        return settings.isEmpty
            ? [.note(Words.text("palette-empty-type-setting", fallback: "Type to find a setting"))]
            : [.label(title: Words.text("folder-picker-recent", fallback: "Recent"), count: nil)] + results(settings)
    }

    /// The Terminals scope with nothing typed: under who needs input first.
    private static func terminalsByState(_ commands: [Command]) -> [PaletteRow] {
        let terminals = commands.filter { $0.group == .terminals }
        var rows: [PaletteRow] = []
        for (title, states) in urgency {
            let members = terminals.filter { states.contains(activity(of: $0)) }
            guard !members.isEmpty else { continue }
            rows += [.label(title: title, count: members.count)] + results(members)
        }
        return rows
    }

    private static var urgency: [(String, [Activity])] {
        [
            (Words.text("palette-state-needs-input", fallback: "Needs input"), [.needsInput]), (Words.text("palette-state-working", fallback: "Working"), [.working]),
            (Words.text("palette-state-finished", fallback: "Finished"), [.finished, .failed]), (Words.text("palette-state-idle", fallback: "Idle"), [.idle]),
        ]
    }

    private static func activity(of command: Command) -> Activity {
        if case .status(let activity, _) = command.lead { activity } else { .idle }
    }

    /// The Actions scope with nothing typed: the actions in their groups, the shells to open a terminal with at the end of
    /// Terminal's, the trusted project's own between View and App (core `actions_by_section`).
    private static func actionsBySection(_ commands: [Command], project: String?) -> [PaletteRow] {
        func section(_ section: ActionSection) -> [PaletteRow] {
            let members = commands.filter { $0.group == .actions && $0.section == section }
            return members.isEmpty ? [] : [.label(title: section.title, count: nil)] + results(members)
        }
        let projectActions = commands.filter { if case .projectAction? = $0.target { true } else { false } }
        let ours: [PaletteRow] = projectActions.isEmpty ? [] : [.projectLabel(project.map { Words.text("palette-project-named", ["project": .text($0)], fallback: "Project · \($0)") } ?? Words.text("palette-project", fallback: "Project"))] + results(projectActions)
        return section(.terminal) + section(.view) + ours + section(.app) + section(.profiles)
    }

    /// A query, or a narrowed scope's whole list: every group that found something, in the palette's order,
    /// each cut to its first few unless the scope is its own.
    private static func found(scope: PaletteScope, query: String, commands: [Command]) -> [PaletteRow] {
        PaletteGroup.allCases.filter { scope.group == nil || scope.group == $0 }.flatMap { group -> [PaletteRow] in
            let ranked = rank(query, commands.filter { $0.group == group }, mixed: scope == .all)
            guard !ranked.isEmpty else { return [] }
            let shown = scope == .all ? Array(ranked.prefix(PaletteGroup.unscopedMax)) : ranked
            return [.label(title: group.title, count: ranked.count)] + shown.map(PaletteRow.result)
        }
    }

    /// Best first; equal scores keep their order so results do not shuffle while typing. A result the core already
    /// matched (a setting) keeps its match and order, by score where it is mixed with other groups.
    private static func rank(_ query: String, _ commands: [Command], mixed: Bool) -> [PaletteResult] {
        guard !query.isEmpty else { return commands.map { PaletteResult(command: $0, ranges: []) } }
        if commands.allSatisfy({ $0.match != nil }), !commands.isEmpty {
            let found = commands.enumerated().map { ($0.offset, $0.element, $0.element.match ?? Match(score: 0, ranges: [])) }
            let ordered = mixed ? found.sorted { ($0.2.score, $1.0) > ($1.2.score, $0.0) } : found
            return ordered.map { PaletteResult(command: $0.1, ranges: $0.2.ranges) }
        }
        let ranked = PaletteSearch.rank(query, commands.map { PaletteCandidate(title: $0.title, other: $0.searchText) })
        return ranked.map { PaletteResult(command: commands[$0.index], ranges: $0.match.ranges) }
    }
}
