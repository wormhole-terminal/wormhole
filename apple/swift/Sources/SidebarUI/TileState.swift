// What a terminal's tile says it is doing, in the GPUI app's own words (`StatusGlyph`): a light circling the
// edge while it works, an amber edge and a "!" while it needs you, a badge for finished, failed and unseen
// output, and nothing at all for a quiet shell.

import WorkspaceModel

enum TileState: Sendable, Equatable {
    case shell, working, needsYou, done, failed, unread

    init(_ terminal: TerminalSummary) {
        switch terminal.activity {
        case .idle: self = terminal.unread > 0 ? .unread : .shell
        case .working: self = .working
        case .needsInput: self = .needsYou
        case .finished: self = .done
        case .failed: self = .failed
        }
    }

    /// Loudest first: core's `Urgency` order, which a folded head sorts its tiles by.
    var urgency: Int {
        switch self {
        case .needsYou: 0
        case .failed: 1
        case .working: 2
        case .unread: 4
        case .done: 6
        case .shell: 7
        }
    }

    /// Needs the person now: what a folded head's dot stands for.
    var isAlert: Bool { self == .needsYou || self == .failed }
}
