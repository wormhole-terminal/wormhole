// Find over a terminal: the Rust search (`wormhole_term::find`) through the source's `TerminalScrollback` covers the
// whole history with case, whole-word and regex as the GPUI app does, off the main thread; matches are painted by the
// view after each frame while the bar is open. A terminal without a scrollback (the mock sources) is searched on the
// rows it shows, with `MockSearching`'s matcher.

import TerminalMetal
import TerminalUI

@MainActor
final class ScreenSearching: TerminalSearching {
    private weak var view: MetalTerminalView?
    private let visibleRows = MockSearching(lines: [])

    init(view: MetalTerminalView) { self.view = view }

    private var finder: (any TerminalFinding)? { view?.source.scrollback }

    func search(_ query: FindQuery) async -> FindOutcome {
        guard let finder else { return await searchVisibleRows(query) }
        let options = query.options
        // The engine's lock is held only to copy the text; the search itself runs on this copy, off the main thread.
        let found = await Task.detached(priority: .userInitiated) {
            finder.find(query.text, matchCase: options.matchCase, wholeWord: options.wholeWord, regex: options.regex)
        }.value
        view?.requestRedraw()
        return found
    }

    func reveal(match index: Int) {
        guard let finder else { return visibleRows.reveal(match: index) }
        finder.findReveal(match: index)
        view?.requestRedraw()
    }

    func ticks() -> [Double] { finder?.findTicks() ?? visibleRows.ticks() }
    func selectionText() -> String? { view?.source.interaction?.selectionText() }
    var hasNewOutput: Bool { finder?.findHasNewOutput ?? false }

    func setActive(_ active: Bool) {
        view?.findActive = active
        if !active { finder?.findClear() }
    }

    private func searchVisibleRows(_ query: FindQuery) async -> FindOutcome {
        if let renderer = view?.renderer {
            visibleRows.lines = (0..<renderer.gridSize.rows).map { renderer.text(ofRow: $0) }
        }
        return await visibleRows.search(query)
    }
}

extension TerminalOverlay {
    /// Esc or the close button: the bar goes and the keys go back to the terminal, as GPUI's find bar gives them back.
    func closeFind() {
        find.close()
        if let view { view.window?.makeFirstResponder(view) }
    }
}
