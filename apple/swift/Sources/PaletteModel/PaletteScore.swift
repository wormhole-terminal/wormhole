// A whole query against one command: every word has to be found, and where it was found sets its weight.

import Foundation

extension PaletteSearch {
    /// A word found only in a result's subtitle counts this share of what it would in the title.
    private static let otherShare = 2

    /// Every word of `query` has to be found, in the title (marked) or in `other` (not marked).
    public static func score(_ query: String, title: String, other: [String] = []) -> Match? {
        var total = Match(score: 0, ranges: [])
        for word in query.split(whereSeparator: \.isWhitespace) {
            let found: Match
            if let inTitle = fuzzy(String(word), in: title) {
                found = inTitle
            } else if let share = other.compactMap({ fuzzy(String(word), in: $0)?.score }).max() {
                found = Match(score: share / otherShare, ranges: [])
            } else {
                return nil
            }
            total.score += found.score
            total.ranges += found.ranges
        }
        total.ranges.sort { $0.lowerBound < $1.lowerBound }
        var merged: [Range<Int>] = []
        for r in total.ranges {
            if let last = merged.last, r.lowerBound <= last.upperBound {
                merged[merged.count - 1] = last.lowerBound..<max(last.upperBound, r.upperBound)
            } else {
                merged.append(r)
            }
        }
        total.ranges = merged
        return total
    }
}
