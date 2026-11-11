// The palette's fuzzy match: core's (`view/search.rs`: every word of a query found in a result's title, marked, or
// in its other texts, ranked best first), behind this seam. Linked to the Rust core, `PlatformBridge` installs
// core's matcher; a build without the FFI keeps `ContainsMatcher`, which finds the same results by substring and
// is there for previews and tests, not to rank like the app does.

import Foundation
import Synchronization

/// How well a text matched, and which characters to mark (offsets into the text's `Character`s).
public struct Match: Sendable, Equatable {
    public var score: Int
    public var ranges: [Range<Int>]
    public init(score: Int, ranges: [Range<Int>]) { (self.score, self.ranges) = (score, ranges) }
}

/// What a result is searched by: its title, which the palette marks, and other texts that find it too (a
/// terminal's folder) without being marked.
public struct PaletteCandidate: Sendable, Equatable {
    public var title: String
    public var other: [String]
    public init(title: String, other: [String] = []) { (self.title, self.other) = (title, other) }
}

/// One candidate a query found: its place in what was asked about, and how.
public struct PaletteRanked: Sendable, Equatable {
    public var index: Int
    public var match: Match
    public init(index: Int, match: Match) { (self.index, self.match) = (index, match) }
}

public protocol PaletteMatching: Sendable {
    /// The candidates `query` finds, best first; equal ones keep their order so results do not shuffle while
    /// typing. Every word has to be found.
    func rank(_ query: String, _ candidates: [PaletteCandidate]) -> [PaletteRanked]
}

public enum PaletteSearch {
    private static let installed = Mutex<any PaletteMatching>(ContainsMatcher())

    /// Makes `matcher` the palette's. Call once, at launch.
    public static func install(_ matcher: any PaletteMatching) { installed.withLock { $0 = matcher } }

    public static func rank(_ query: String, _ candidates: [PaletteCandidate]) -> [PaletteRanked] {
        installed.withLock { $0 }.rank(query, candidates)
    }
}

/// Every word of the query as a substring of the title (marked) or of another text, earlier in the title first.
public struct ContainsMatcher: PaletteMatching {
    public init() {}

    public func rank(_ query: String, _ candidates: [PaletteCandidate]) -> [PaletteRanked] {
        let words = query.split(whereSeparator: \.isWhitespace).map(String.init)
        let found = candidates.enumerated().compactMap { index, candidate -> PaletteRanked? in
            var match = Match(score: 0, ranges: [])
            for word in words {
                if let at = candidate.title.range(of: word, options: [.caseInsensitive, .diacriticInsensitive]) {
                    let start = candidate.title.distance(from: candidate.title.startIndex, to: at.lowerBound)
                    match.score += 100 - min(start, 99)
                    match.ranges.append(start..<start + word.count)
                } else if candidate.other.contains(where: { $0.localizedCaseInsensitiveContains(word) }) {
                    match.score += 50
                } else {
                    return nil
                }
            }
            return PaletteRanked(index: index, match: match)
        }
        return found.sorted { ($0.match.score, $1.index) > ($1.match.score, $0.index) }
    }
}
