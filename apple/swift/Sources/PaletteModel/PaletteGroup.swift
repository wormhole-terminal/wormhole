// A kind of result, in the order the unscoped palette lists them (core `search::Group::ORDER`).

import Foundation
import WordsModel

public enum PaletteGroup: Int, Sendable, CaseIterable {
    case settings, actions, terminals, folders, commits

    public var title: String {
        switch self {
        case .settings: Words.text("palette-group-settings", fallback: "Settings")
        case .actions: Words.text("palette-group-actions", fallback: "Actions")
        case .terminals: Words.text("palette-group-terminals", fallback: "Terminals")
        case .folders: Words.text("palette-group-folders", fallback: "Folders")
        case .commits: Words.text("palette-group-commits", fallback: "Commits")
        }
    }

    /// How many results of a group the unscoped palette shows; the label still counts all of them
    /// and the group's own scope lists them all (core `search::GROUP_MAX`).
    public static let unscopedMax = 5
}
