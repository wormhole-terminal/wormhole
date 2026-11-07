// Find over what a terminal shows right now, until the Rust search (`wormhole_term::find`) backs the protocol with
// the scrollback too. The matching is `MockSearching`'s (literal, whole word, regex, case); only the lines are the
// terminal's: its visible rows, read when a search starts.

import TerminalMetal
import TerminalUI

@MainActor
final class ScreenSearching: TerminalSearching {
    private weak var view: MetalTerminalView?
    private let matcher = MockSearching(lines: [])

    init(view: MetalTerminalView) { self.view = view }

    func search(_ query: FindQuery) async -> FindOutcome {
        if let renderer = view?.renderer {
            matcher.lines = (0..<renderer.gridSize.rows).map { renderer.text(ofRow: $0) }
        }
        return await matcher.search(query)
    }

    func reveal(match index: Int) { matcher.reveal(match: index) }
    func ticks() -> [Double] { matcher.ticks() }
    func selectionText() -> String? { nil }
    var hasNewOutput: Bool { false }
}
