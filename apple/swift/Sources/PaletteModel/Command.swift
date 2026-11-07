// One result of the palette: what it is called, what leads it, the words beside it, its key and what it does.

import Foundation
import WorkspaceModel

/// What stands in a result's lead column: a terminal's status, or an icon (an SF Symbol name).
public enum Lead: Sendable, Equatable {
    case symbol(String)
    case status(Activity)
}

/// The groups of the Actions scope with nothing typed (core `palette::Section`).
public enum ActionSection: Int, Sendable, CaseIterable {
    case terminal, view

    public var title: String {
        switch self {
        case .terminal: "Terminal"
        case .view: "View"
        }
    }
}

public struct Command: Sendable, Equatable, Identifiable {
    public var id: String
    public var title: String
    /// After the title, in caption text: where, what it is doing.
    public var detail: String?
    /// The detail in the Terminals scope, where the window and the folder are the point.
    public var scopedDetail: String?
    public var group: PaletteGroup
    public var lead: Lead
    public var shortcut: Shortcut?
    /// What running it sends.
    public var action: WorkspaceAction?
    /// The title in medium weight rather than regular: a terminal's name.
    public var isStrong: Bool
    /// More words a query may find it by, which are not marked.
    public var searchText: [String]
    public var section: ActionSection?

    public init(
        id: String, title: String, detail: String? = nil, scopedDetail: String? = nil, group: PaletteGroup = .actions, lead: Lead,
        shortcut: Shortcut? = nil, action: WorkspaceAction? = nil, isStrong: Bool = false,
        searchText: [String] = [], section: ActionSection? = nil
    ) {
        self.id = id
        self.title = title
        self.detail = detail
        self.scopedDetail = scopedDetail
        self.group = group
        self.lead = lead
        self.shortcut = shortcut
        self.action = action
        self.isStrong = isStrong
        self.searchText = searchText
        self.section = section
    }
}
