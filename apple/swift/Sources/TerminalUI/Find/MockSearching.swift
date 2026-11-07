// A searcher over a list of lines, for previews, the gallery and tests: literal or regex matching, no terminal.

import Foundation
import TerminalCore

@MainActor
public final class MockSearching: TerminalSearching {
    public var lines: [String]
    public var selection: String?
    public private(set) var revealed: [Int] = []
    public var hasNewOutput = false
    private var matches: [Int] = []

    public init(lines: [String] = MockSearching.sample, selection: String? = nil) {
        (self.lines, self.selection) = (lines, selection)
    }

    public func search(_ query: FindQuery) async -> FindOutcome {
        matches = []
        guard !query.text.isEmpty else { return FindOutcome() }
        let pattern = query.options.regex ? query.text : NSRegularExpression.escapedPattern(for: query.text)
        let bounded = query.options.wholeWord ? "\\b\(pattern)\\b" : pattern
        guard let regex = try? NSRegularExpression(
            pattern: bounded, options: query.options.matchCase ? [] : [.caseInsensitive])
        else { return FindOutcome(invalid: true) }
        for (row, line) in lines.enumerated() {
            let range = NSRange(line.startIndex..., in: line)
            matches += Array(repeating: row, count: regex.numberOfMatches(in: line, range: range))
        }
        return FindOutcome(count: matches.count, nearest: matches.isEmpty ? nil : matches.count - 1)
    }

    public func reveal(match index: Int) { revealed.append(index) }
    public func setActive(_ active: Bool) {}
    public func selectionText() -> String? { selection }
    public func ticks() -> [Double] { matches.map { Double($0) / Double(max(lines.count, 1)) } }

    public static let sample = [
        "   Compiling wormhole-core v1.6.3", "   Compiling wormhole-term v1.6.3", "warning: unused import",
        "    Finished `dev` profile in 41s", "$ cargo test", "running 211 tests", "test result: ok. 211 passed",
    ]
}
