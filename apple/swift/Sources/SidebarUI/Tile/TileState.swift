// What a terminal's tile says it is doing, in the GPUI app's own words (`StatusGlyph`, `row/tile.rs` `glyph_for`): a
// light circling the edge while an agent works, an amber edge and a "!" while it needs you, a badge for finished,
// failed, the bell and unseen output, and nothing at all for a quiet shell. Only an agent's own state is told in
// the tile; a plain terminal's command runs without one, the way the GPUI app draws it.

import WorkspaceModel

/// The cases are in core's `Urgency` order, loudest first (`view/sidebar/urgency.rs`): what a folded head and a tab
/// switcher pick by.
enum TileState: Int, Sendable, Equatable, Comparable {
    case needsYou, failed, working, bell, unread, running, done, shell

    init(_ terminal: TerminalSummary) {
        let told = terminal.agent != nil
        switch terminal.activity {
        case .needsInput where told: self = .needsYou
        case .working where told: self = .working
        case .finished where told: self = .done
        case .failed: self = .failed
        case .idle, .working, .needsInput, .finished:
            if !told, terminal.runningCommand != nil { self = .running }
            else if !told, terminal.lastFailure != nil { self = .failed }
            else if !told, terminal.unread > 0 { self = .unread }
            else { self = .shell }
        }
        // A terminal with nothing louder to say says its bell.
        if terminal.hasBell, self == .shell || self == .running { self = .bell }
    }

    static func < (lhs: TileState, rhs: TileState) -> Bool { lhs.rawValue < rhs.rawValue }

    /// Needs the person now: what a folded head's dot stands for.
    var isAlert: Bool { self == .needsYou || self == .failed }
}
