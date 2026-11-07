// The palette's fuzzy match, ported from core's `view/search.rs` so both apps find the same things: a word's
// letters in order, the first at a word start; word starts and runs score higher, and early beats late.

import Foundation

/// How well a text matched, and which characters to mark (offsets into the text's `Character`s).
public struct Match: Sendable, Equatable {
    public var score: Int
    public var ranges: [Range<Int>]
}

public enum PaletteSearch {
    private static let matchPoints = 16
    private static let wordStart = 10
    private static let textStart = 6
    private static let run = 12
    private static let gap = 1
    private static let leadGapMax = 8
    private static let none = Int.min / 2

    private struct Glyph {
        var lower: Character
        var startsWord: Bool
    }

    private static func glyphs(_ text: String) -> [Glyph] {
        var out: [Glyph] = []
        var previous: Character?
        for c in text {
            let starts: Bool
            if let p = previous {
                starts = (!(p.isLetter || p.isNumber) && (c.isLetter || c.isNumber)) || (p.isLowercase && c.isUppercase)
            } else {
                starts = true
            }
            out.append(Glyph(lower: lower(c), startsWord: starts))
            previous = c
        }
        return out
    }

    private static func lower(_ c: Character) -> Character { c.lowercased().first ?? c }

    /// `word` in `text`: the best placement that starts a word, else the word as typed anywhere. nil: not there.
    public static func fuzzy(_ word: String, in text: String) -> Match? {
        let query = word.map(lower)
        if query.isEmpty { return Match(score: 0, ranges: []) }
        let text = glyphs(text)
        guard query.count <= text.count else { return nil }
        let contiguous: ([Int]) -> Bool = { zip($0, $0.dropFirst()).allSatisfy { $1 == $0 + 1 } }
        var found = align(query, text, startsWord: false)
        if let f = found, !(text[f.picks[0]].startsWord || contiguous(f.picks)) { found = nil }
        guard let (score, picks) = found ?? align(query, text, startsWord: true) ?? asTyped(query, text) else {
            return nil
        }
        var ranges: [Range<Int>] = []
        for j in picks {
            if let last = ranges.last, last.upperBound == j {
                ranges[ranges.count - 1] = last.lowerBound..<(j + 1)
            } else {
                ranges.append(j..<(j + 1))
            }
        }
        return Match(score: score, ranges: ranges)
    }

    private typealias Alignment = (score: Int, picks: [Int])

    private static func firstScore(_ text: [Glyph], _ j: Int) -> Int {
        let bonus = j == 0 ? textStart + wordStart : (text[j].startsWord ? wordStart : 0)
        return matchPoints + bonus - min(j * gap, leadGapMax)
    }

    /// Dynamic programming over (query character, text position): the best score with the query's first
    /// `i + 1` characters placed and the last at `j`.
    private static func align(_ query: [Character], _ text: [Glyph], startsWord: Bool) -> Alignment? {
        let (m, n) = (query.count, text.count)
        var best = Array(repeating: Array(repeating: none, count: n), count: m)
        var from = Array(repeating: Array(repeating: -1, count: n), count: m)
        for j in 0..<n where text[j].lower == query[0] && (!startsWord || text[j].startsWord) {
            best[0][j] = firstScore(text, j)
        }
        for i in 1..<max(m, 1) {
            var skipped = none
            var skippedAt = -1
            for j in i..<max(n, i) {
                if text[j].lower == query[i] {
                    let consecutive = best[i - 1][j - 1]
                    // After a gap the next character has to start a word ("sp" in "sidebar polish").
                    let skip = text[j].startsWord ? skipped : none
                    let (prev, at) =
                        consecutive > none && consecutive + run >= skip ? (consecutive + run, j - 1) : (skip, skippedAt)
                    if prev > none {
                        best[i][j] = prev + matchPoints + (text[j].startsWord ? wordStart : 0)
                        from[i][j] = at
                    }
                }
                let decayed = skipped > none ? skipped - gap : none
                let here = best[i - 1][j - 1]
                if here > none && here - gap > decayed {
                    (skipped, skippedAt) = (here - gap, j - 1)
                } else {
                    skipped = decayed
                }
            }
        }
        // Ties go to the earlier end, like the Rust version.
        guard let end = (0..<n).max(by: { (best[m - 1][$0], n - $0) < (best[m - 1][$1], n - $1) }),
            best[m - 1][end] > none
        else { return nil }
        var picks = Array(repeating: 0, count: m)
        var j = end
        for i in stride(from: m - 1, through: 0, by: -1) {
            picks[i] = j
            j = from[i][j]
        }
        return (best[m - 1][end], picks)
    }

    private static func asTyped(_ query: [Character], _ text: [Glyph]) -> Alignment? {
        guard let start = (0...(text.count - query.count)).first(where: { j in
            query.indices.allSatisfy { text[j + $0].lower == query[$0] }
        }) else { return nil }
        let score = firstScore(text, start) + (query.count - 1) * (matchPoints + run)
        return (score, Array(start..<(start + query.count)))
    }
}
