// A kind of result, in the order the unscoped palette lists them (core `search::Group::ORDER`).

import Foundation

public enum PaletteGroup: Int, Sendable, CaseIterable {
    case settings, actions, terminals, folders, commits

    public var title: String {
        switch self {
        case .settings: "Settings"
        case .actions: "Actions"
        case .terminals: "Terminals"
        case .folders: "Folders"
        case .commits: "Commits"
        }
    }

    /// How many results of a group the unscoped palette shows; the label still counts all of them
    /// and the group's own scope lists them all (core `search::GROUP_MAX`).
    public static let unscopedMax = 5
}
