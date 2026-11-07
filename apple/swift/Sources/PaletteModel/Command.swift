// One entry of the palette: what it is called, where it is filed, its key and what it does.

import Foundation
import WorkspaceModel

/// The palette's groups, in the order an unfiltered palette lists them (core `search::Group::ORDER`).
public enum CommandGroup: Int, Sendable, CaseIterable {
    case actions, terminals

    public var title: String {
        switch self {
        case .actions: "Actions"
        case .terminals: "Terminals"
        }
    }
}

/// Where the shortcut sheet files a command (the section names of core's `keys/keymap/sheet.rs`).
public enum CommandCategory: Int, Sendable, CaseIterable {
    case terminals, view, findAnything

    public var title: String {
        switch self {
        case .terminals: "Windows & terminals"
        case .view: "View & panels"
        case .findAnything: "Find anything"
        }
    }
}

public struct Command: Sendable, Equatable, Identifiable {
    public var id: String
    public var title: String
    public var subtitle: String?
    public var group: CommandGroup
    public var category: CommandCategory
    /// An SF Symbol name for the row's icon.
    public var symbol: String
    public var shortcut: Shortcut?
    /// What running it sends; nil for the entries the window handles itself (open the palette).
    public var action: WorkspaceAction?

    public init(
        id: String, title: String, subtitle: String? = nil, group: CommandGroup = .actions,
        category: CommandCategory, symbol: String, shortcut: Shortcut? = nil, action: WorkspaceAction? = nil
    ) {
        self.id = id
        self.title = title
        self.subtitle = subtitle
        self.group = group
        self.category = category
        self.symbol = symbol
        self.shortcut = shortcut
        self.action = action
    }
}
