// What the palette lists for a typed query: the scope it asks for, then the commands found, best first.

import Foundation

public struct PaletteResult: Sendable, Equatable, Identifiable {
    public var command: Command
    /// Character offsets into `command.title` to mark.
    public var ranges: [Range<Int>]
    public var id: String { command.id }
}

/// A prefix typed first narrows the palette to one group (core `search::parse`): `@` terminals, `>` actions.
public enum PaletteScope: Sendable, Equatable {
    case all, terminals, actions

    public var group: PaletteGroup? {
        switch self {
        case .all: nil
        case .terminals: .terminals
        case .actions: .actions
        }
    }

    public var title: String? {
        switch self {
        case .all: nil
        case .terminals: "Terminals"
        case .actions: "Actions"
        }
    }

    /// The scope `input` asks for and the query after its prefix.
    public static func parse(_ input: String) -> (scope: PaletteScope, query: String) {
        let trimmed = input.drop(while: \.isWhitespace)
        let scope: PaletteScope =
            switch trimmed.first {
            case "@": .terminals
            case ">": .actions
            default: .all
            }
        guard scope != .all else { return (.all, input) }
        return (scope, String(trimmed.dropFirst().drop(while: \.isWhitespace)))
    }
}

extension PaletteSearch {
    /// The commands `input` finds, best first; equal scores keep the catalog's order so results do not
    /// shuffle while typing. An empty query lists all of the scope in the catalog's order, grouped.
    public static func results(for input: String, in commands: [Command]) -> [PaletteResult] {
        let (scope, query) = PaletteScope.parse(input)
        let pool = commands.filter { scope.group == nil || $0.group == scope.group }
        guard !query.allSatisfy(\.isWhitespace) else {
            return pool.enumerated()
                .sorted { ($0.element.group.rawValue, $0.offset) < ($1.element.group.rawValue, $1.offset) }
                .map { PaletteResult(command: $0.element, ranges: []) }
        }
        return pool.enumerated()
            .compactMap { offset, command -> (Int, Int, PaletteResult)? in
                guard let match = score(query, title: command.title, other: command.subtitle.map { [$0] } ?? [])
                else { return nil }
                return (match.score, offset, PaletteResult(command: command, ranges: match.ranges))
            }
            .sorted { ($1.0, $0.1) < ($0.0, $1.1) }
            .map(\.2)
    }
}
