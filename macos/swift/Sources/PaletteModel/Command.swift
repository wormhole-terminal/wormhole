// One result of the palette: what it is called, what leads it, the words beside it, its key and what it does.

import Foundation
import WordsModel
import WorkspaceModel

/// What stands in a result's lead column: a terminal's status, or an icon (an SF Symbol name).
public enum Lead: Sendable, Equatable {
    case symbol(String)
    /// `told`: an agent or a program that reports gives the state; without one a needs-input is a bell, working a
    /// plain command and idle a bare shell (`StatusGlyph::of`).
    case status(Activity, told: Bool)
}

/// The groups of the Actions scope with nothing typed (core `keymap::palette::Section`).
public enum ActionSection: Int, Sendable, CaseIterable {
    case terminal, view, app
    /// The profile rows after App's (core's palette has them only with another profile to go to).
    case profiles

    public var title: String {
        switch self {
        case .terminal: Words.text("palette-section-terminal", fallback: "Terminal")
        case .view: Words.text("palette-section-view", fallback: "View")
        case .app: Words.text("palette-section-app", fallback: "App")
        case .profiles: Words.text("palette-section-profiles", fallback: "Profiles")
        }
    }

    /// The section the core names by its title (its English, whatever language the interface is in).
    public init?(title: String) {
        switch title {
        case "Terminal": self = .terminal // text: the core's section name, matched not shown
        case "View": self = .view
        case "App": self = .app
        case "Profiles": self = .profiles
        default: return nil
        }
    }
}

public struct Command: Sendable, Equatable, Identifiable {
    /// What running a result other than an action or a terminal does: open a setting's row, show a commit.
    public enum Target: Sendable, Equatable {
        case setting(String)
        case commit(String)
        /// A profile by its place in the list.
        case profile(ProfileCommand, Int)
        /// A terminal of any window, a folder with terminals (or one lately worked in), by path.
        case terminal(TerminalID)
        case folder(String)
        /// The front terminal's project's action, by id, and a new terminal running the shell with this key.
        case projectAction(String)
        case newTerminalWith(String)
    }

    public enum ProfileCommand: Sendable, Equatable { case switchTo, openWindowIn, moveWindowTo }

    public var id: String
    public var title: String
    /// After the title, in caption text: where, what it is doing.
    public var detail: String?
    /// The detail in the Terminals scope, where the window and the folder are the point.
    public var scopedDetail: String?
    /// A line under the title in that scope: what the agent asks. Makes the row two lines tall.
    public var second: String?
    /// The detail is a command: set in the code font (a project's action, a shell).
    public var detailIsCode = false
    public var group: PaletteGroup
    public var lead: Lead
    public var shortcut: Shortcut?
    /// At the right, in caption text: a setting's value, a commit's short hash (`trailingIsCode`).
    public var trailing: String?
    public var trailingIsCode = false
    /// What running it sends.
    public var action: WorkspaceAction?
    /// A keymap action the window carries out (`splitRight`): the router runs it, as the menu and the key do.
    public var appAction: String?
    public var target: Target?
    /// The title in medium weight rather than regular: a terminal's name.
    public var isStrong: Bool
    /// More words a query may find it by, which are not marked.
    public var searchText: [String]
    public var section: ActionSection?
    /// How the core already matched it (a setting is found by the settings index, which also reads its keywords): used
    /// instead of ranking the title again.
    public var match: Match?

    public init(
        id: String, title: String, detail: String? = nil, scopedDetail: String? = nil, second: String? = nil, group: PaletteGroup = .actions, lead: Lead,
        shortcut: Shortcut? = nil, trailing: String? = nil, action: WorkspaceAction? = nil, appAction: String? = nil,
        target: Target? = nil, isStrong: Bool = false, searchText: [String] = [], section: ActionSection? = nil,
        match: Match? = nil
    ) {
        self.id = id
        self.title = title
        self.detail = detail
        self.scopedDetail = scopedDetail
        self.second = second
        self.group = group
        self.lead = lead
        self.shortcut = shortcut
        self.trailing = trailing
        self.action = action
        self.appAction = appAction
        self.target = target
        self.isStrong = isStrong
        self.searchText = searchText
        self.section = section
        self.match = match
    }
}
