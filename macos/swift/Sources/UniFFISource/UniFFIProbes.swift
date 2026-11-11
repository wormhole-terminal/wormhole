// `TerminalProbes` over the Rust terminal: folds opened, a shortened span revealed, a rule's file found. One seam call
// each; the decisions are Rust's (`probes/terminal.rs`).

import Foundation
import TerminalCore
import WormholeBindings

final class UniFFIProbes: TerminalCore.TerminalProbes {
    private let terminal: WormholeBindings.Terminal
    init(_ terminal: WormholeBindings.Terminal) { self.terminal = terminal }

    var hasFolds: Bool { terminal.hasFolds() }
    func foldToggle(_ id: UInt64) -> Bool { terminal.foldToggle(id: id) }
    func foldBlockOutput(_ block: UInt64) -> Bool { terminal.foldBlockOutput(block: block) }
    func foldShowAll() -> Int { Int(terminal.foldShowAll()) }
    func foldShownWords(lines: Int) -> String { terminal.foldShownWords(lines: UInt32(lines)) }

    func setReveal(row: Int?, column: Int?, alt: Bool) -> Bool {
        terminal.setReveal(row: row.map(UInt32.init), col: column.map(UInt32.init), alt: alt)
    }

    func resolvePath(_ text: String) -> TerminalCore.ResolvedFile {
        let found = terminal.resolvePath(text: text)
        return TerminalCore.ResolvedFile(path: found.path, line: found.line.map(Int.init), words: found.words)
    }

    func openFile(path: String, line: Int?, bundle: String, appFolder: String?) -> Bool {
        openFileRun(bundle: bundle, path: path, line: line.map(UInt32.init), appFolder: appFolder)
    }
}

/// A test instance's launches of the core (a file in an editor at a line) are written to `file` instead of started.
public func recordLaunches(to file: String?) { openFileRecordTo(file: file) }
