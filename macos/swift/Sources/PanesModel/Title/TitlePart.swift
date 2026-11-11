// The parts the terminal's title row can show (`view/layout/title_parts.rs`): the person's own arrangement,
// as a toolbar's items are. Each is drawn only when there is something to show and the row has room.

import Foundation

public enum TitlePart: String, Sendable, CaseIterable, Equatable, Hashable {
    /// The terminal's folder, cut in its middle where it is long.
    case folder
    /// The folder's branch, as a chip.
    case branch
    /// A dot while the tree has uncommitted changes, and the commits ahead and behind.
    case branchStatus = "branch_status"
    /// The terminal's status tile and its name.
    case terminal
    /// How the terminal is doing ("working · 4m"), when the status bar doesn't say it.
    case terminalStatus = "terminal_status"
    /// The flexible space between the row's two ends.
    case space
    /// A coding agent's model and how full its context window is.
    case agentGauge = "agent_gauge"
    /// The ports the terminal's programs listen on.
    case ports
    /// The branch's pull request and its checks.
    case pullRequest = "pull_request"
    /// Open in…: the folder in an editor, and the menu of the others.
    case openIn = "open_in"
    /// The two split buttons.
    case splits

    /// How a part joins the parts beside it.
    public enum Kind: Sendable, Equatable {
        /// A step of the breadcrumb (folder › branch › terminal): a chevron separates two of them.
        case crumb
        /// Words or marks that belong to the crumb before them, with no chevron of their own.
        case inline
        /// A chip that stands on its own.
        case chip
        /// An icon button; buttons side by side are flush.
        case button
        case space
    }

    public var kind: Kind {
        switch self {
        case .folder, .branch, .terminal: .crumb
        case .branchStatus, .terminalStatus: .inline
        case .agentGauge, .ports, .pullRequest: .chip
        case .openIn, .splits: .button
        case .space: .space
        }
    }

    /// Whether the person can hide it: every part but the space.
    public var hideable: Bool { self != .space }
}
