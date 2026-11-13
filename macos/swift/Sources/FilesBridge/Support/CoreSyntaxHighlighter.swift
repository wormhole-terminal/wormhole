// Syntax colours from the core: tree-sitter reads the whole text with the grammar the file's name says (`wormhole-ffi`'s
// `syntax`, FFI-CONTRACT.md "Syntax colours"). A file without a grammar, a text over the core's cap and one that takes too long
// come back uncoloured. Blocking: the stores call it off the main actor, once per text they show.

import PanelModel
import WormholeBindings

/// The names of the keys a run counts by, asked of the core once.
private let keyNames: [String] = syntaxKeys()

public struct CoreSyntaxHighlighter: SyntaxHighlighting {
    public init() {}

    public func highlight(path: String, lines: [String]) -> [[SyntaxSpan]] {
        let plain = lines.map { _ in [SyntaxSpan]() }
        guard !lines.isEmpty, let colored = syntaxLines(text: lines.joined(separator: "\n"), path: path), colored.count == lines.count else { return plain }
        return zip(lines, colored).map { line, runs in
            SyntaxSpan.inCharacters(
                runs.compactMap { run -> (range: Range<Int>, key: String)? in
                    Int(run.key) < keyNames.count ? (range: Int(run.start)..<Int(run.end), key: keyNames[Int(run.key)]) : nil
                }, of: line)
        }
    }
}
