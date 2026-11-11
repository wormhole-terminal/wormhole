// What the palette searches: everything, or one kind. A prefix typed first picks it, Tab steps through them
// (core `search::Scope`). Files has no scope there either (backlog).

import Foundation
import WordsModel

public enum PaletteScope: Sendable, Equatable, CaseIterable {
    case all, terminals, folders, commits, settings, actions

    /// The scope row, left to right: `allCases` is in that order.
    public static var row: [PaletteScope] { allCases }

    public var prefix: Character? {
        switch self {
        case .all: nil
        case .terminals: "@"
        case .folders: "~"
        case .commits: "#"
        case .settings: ","
        case .actions: ">"
        }
    }

    /// Its name on the scope row and on the search field's chip.
    public var title: String {
        switch self {
        case .all: Words.text("palette-scope-all", fallback: "All")
        case .terminals: Words.text("palette-group-terminals", fallback: "Terminals")
        case .folders: Words.text("palette-group-folders", fallback: "Folders")
        case .commits: Words.text("palette-group-commits", fallback: "Commits")
        case .settings: Words.text("palette-group-settings", fallback: "Settings")
        case .actions: Words.text("palette-group-actions", fallback: "Actions")
        }
    }

    /// What the empty search field says.
    public var placeholder: String {
        switch self {
        case .all: Words.text("palette-placeholder-all", fallback: "Jump to a terminal, folder, commit or setting…")
        case .terminals: Words.text("palette-placeholder-terminals", fallback: "Terminals in every window…")
        case .folders: Words.text("palette-placeholder-folders", fallback: "Folders with a terminal, and recent ones…")
        case .commits: Words.text("palette-placeholder-commits", fallback: "Commits on this branch…")
        case .settings: Words.text("palette-placeholder-settings", fallback: "Find a setting…")
        case .actions: Words.text("palette-placeholder-actions", fallback: "Run an action…")
        }
    }

    /// The one group a narrowed palette shows.
    public var group: PaletteGroup? {
        switch self {
        case .all: nil
        case .terminals: .terminals
        case .folders: .folders
        case .commits: .commits
        case .settings: .settings
        case .actions: .actions
        }
    }

    /// The scope Tab (forward) or ⇧Tab moves to, wrapping around.
    public func step(forward: Bool) -> PaletteScope {
        let scopes = Self.row
        let at = scopes.firstIndex(of: self) ?? 0
        return scopes[(at + (forward ? 1 : scopes.count - 1)) % scopes.count]
    }

    /// The scope `input` asks for and the query after its prefix. Only a first character counts ("a@b" looks
    /// for "a@b"), and `/` is not a prefix.
    public static func parse(_ input: String) -> (scope: PaletteScope, query: String) {
        let trimmed = input.drop(while: \.isWhitespace)
        guard let first = trimmed.first, let scope = row.first(where: { $0.prefix == first }) else {
            return (.all, input)
        }
        return (scope, String(trimmed.dropFirst().drop(while: \.isWhitespace)))
    }
}
