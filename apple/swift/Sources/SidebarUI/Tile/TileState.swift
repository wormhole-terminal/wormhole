// What a terminal's tile says it is doing, in the GPUI app's own words (`StatusGlyph`, `row/tile.rs` `glyph_for`): a
// light circling the edge while an agent works, an amber edge and a "!" while it needs you, a badge for finished,
// failed, the bell and unseen output, and nothing at all for a quiet shell. Only an agent's own state is told in
// the tile; a plain terminal's command runs without one, the way the GPUI app draws it. The core has folded a plain
// terminal's bell into "needs you" and its running command into "working"; without an agent they read as these.

import WorkspaceModel

/// The cases are in core's `Urgency` order, loudest first (`view/sidebar/urgency.rs`).
enum TileState: Int, Sendable, Equatable {
    case needsYou, failed, working, bell, unread, running, done, shell

    init(_ terminal: TerminalSummary) {
        let told = terminal.agent != nil
        switch terminal.activity {
        case .needsInput: self = told ? .needsYou : .bell
        case .working: self = told ? .working : .running
        case .finished: self = told ? .done : .shell
        case .failed: self = .failed
        case .idle, .exited: self = !told && terminal.unread ? .unread : .shell
        }
        // A terminal with nothing louder to say says its bell.
        if terminal.hasBell, self == .shell || self == .running { self = .bell }
    }
}
