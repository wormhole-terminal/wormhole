// `TerminalScrollback` over the Rust terminal: find, blocks and the minimap are each one seam call with the types
// translated (the generated bindings and TerminalCore name some types alike, so both are written with their module).
// The class is Sendable because Rust's `Terminal` is: the search runs on a background thread through it.

import Foundation
import TerminalCore
import WormholeBindings

final class UniFFIScrollback: TerminalFinding, TerminalBlocks, TerminalMinimap {
    let terminal: WormholeBindings.Terminal
    init(_ terminal: WormholeBindings.Terminal) { self.terminal = terminal }

    // MARK: find

    func find(_ text: String, matchCase: Bool, wholeWord: Bool, regex: Bool) -> TerminalCore.FindOutcome {
        let found = terminal.findSearch(text: text, matchCase: matchCase, wholeWord: wholeWord, regex: regex)
        return TerminalCore.FindOutcome(
            count: Int(found.count), capped: found.capped, invalid: found.invalid, nearest: found.nearest.map(Int.init))
    }

    func findReveal(match index: Int) { terminal.findReveal(index: UInt32(clamping: index)) }

    func findSpans() -> [FindSpan] {
        terminal.findSpans().map {
            FindSpan(row: Int($0.row), first: Int($0.first), last: Int($0.last), isCurrent: $0.current)
        }
    }

    func findFoldWords() -> [TerminalCore.FoldWords] {
        terminal.findFoldWords().map { TerminalCore.FoldWords(row: Int($0.row), label: $0.label, spoken: $0.spoken) }
    }

    func findTicks() -> [Double] { terminal.findTicks() }
    var findHasNewOutput: Bool { terminal.findHasNewOutput() }
    func findClear() { terminal.findClear() }

    // MARK: minimap

    func minimapRead(fits: Int) -> MinimapRead {
        let read = terminal.minimapGeometry(fits: UInt32(clamping: fits))
        return MinimapRead(
            total: Int(read.total), screen: Int(read.screen), fits: Int(read.fits), viewTop: Int(read.viewTop),
            columns: Int(read.columns), stamp: read.stamp)
    }

    func minimapLines() -> [[MinimapRun]] {
        terminal.minimapLines().map { runs in
            runs.map {
                MinimapRun(
                    start: Int($0.start), end: Int($0.end), isText: $0.text,
                    cell: PackedCell(scalar: $0.scalar, fg: $0.fg, bg: $0.bg, flags: CellFlags(rawValue: $0.flags)))
            }
        }
    }

    func scroll(toDisplayOffset offset: Int) { terminal.scrollToDisplayOffset(offset: UInt32(clamping: max(offset, 0))) }
}
