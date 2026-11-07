// The lines of the palette for a scope and a query (core's `command_palette/rows`): a group's label with its
// count, a result with the characters the search found, or a note where a list has nothing. Selection only
// ever lands on a result.

import Foundation
import WorkspaceModel

public enum PaletteRow: Sendable, Equatable, Identifiable {
    case label(title: String, count: Int?)
    case result(PaletteResult)
    case note(String)

    public var id: String {
        switch self {
        case .label(let title, _): "label-\(title)"
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
    /// `query` is the words after any prefix. `recent` is the ids chosen before, newest first.
    public static func rows(
        scope: PaletteScope, query: String, commands: [Command], recent: [String] = []
    ) -> [PaletteRow] {
        let query = query.trimmingCharacters(in: .whitespaces)
        let rows =
            switch (scope, query.isEmpty) {
            case (.all, true): suggestions(commands, recent: recent)
            case (.terminals, true): terminalsByState(commands)
            case (.actions, true): actionsBySection(commands)
            case (.settings, true): [PaletteRow.note("Type to find a setting")]
            default: found(scope: scope, query: query, commands: commands)
            }
        return rows.isEmpty ? [.note(query.isEmpty ? "Nothing here yet" : "Nothing matches")] : rows
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
    private static func suggestions(_ commands: [Command], recent: [String]) -> [PaletteRow] {
        let lookup = Dictionary(commands.map { ($0.id, $0) }, uniquingKeysWith: { first, _ in first })
        let recents = recent.compactMap { lookup[$0] }.filter { $0.group != .actions }.prefix(6)
        var rows: [PaletteRow] = recents.isEmpty ? [] : [.label(title: "Recent", count: nil)] + results(Array(recents))
        let suggested = CommandCatalog.suggested.compactMap { lookup[$0] }
        if !suggested.isEmpty { rows += [.label(title: "Actions", count: nil)] + results(suggested) }
        return rows
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

    private static let urgency: [(String, [Activity])] = [
        ("Needs input", [.needsInput]), ("Working", [.working]), ("Finished", [.finished, .failed]),
        ("Idle", [.idle]),
    ]

    private static func activity(of command: Command) -> Activity {
        if case .status(let activity) = command.lead { activity } else { .idle }
    }

    /// The Actions scope with nothing typed: the actions in their groups.
    private static func actionsBySection(_ commands: [Command]) -> [PaletteRow] {
        ActionSection.allCases.flatMap { section -> [PaletteRow] in
            let members = commands.filter { $0.group == .actions && $0.section == section }
            return members.isEmpty ? [] : [.label(title: section.title, count: nil)] + results(members)
        }
    }

    /// A query, or a narrowed scope's whole list: every group that found something, in the palette's order,
    /// each cut to its first few unless the scope is its own.
    private static func found(scope: PaletteScope, query: String, commands: [Command]) -> [PaletteRow] {
        PaletteGroup.allCases.filter { scope.group == nil || scope.group == $0 }.flatMap { group -> [PaletteRow] in
            let ranked = rank(query, commands.filter { $0.group == group })
            guard !ranked.isEmpty else { return [] }
            let shown = scope == .all ? Array(ranked.prefix(PaletteGroup.unscopedMax)) : ranked
            return [.label(title: group.title, count: ranked.count)] + shown.map(PaletteRow.result)
        }
    }

    /// Best first; equal scores keep their order so results do not shuffle while typing.
    private static func rank(_ query: String, _ commands: [Command]) -> [PaletteResult] {
        guard !query.isEmpty else { return commands.map { PaletteResult(command: $0, ranges: []) } }
        let ranked = PaletteSearch.rank(query, commands.map { PaletteCandidate(title: $0.title, other: $0.searchText) })
        return ranked.map { PaletteResult(command: commands[$0.index], ranges: $0.match.ranges) }
    }
}
