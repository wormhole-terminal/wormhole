// Helping the person type a name: the names that begin with the word being typed (an expression ends in one,
// `mix(primary, surface.` offers `surface.window`, `surface.panel`...), and for a name that doesn't exist the
// closest one that does ("Did you mean `primary`?"), as `studio/completion.rs` and core's `suggest::closest` do.

import Foundation

public enum NameCompletion {
    /// The word an expression ends in, which a completion would replace.
    public static func trailingWord(of text: String) -> Substring {
        let isName = { (c: Character) in c.isLetter || c.isNumber || c == "." || c == "_" }
        let start = text.reversed().firstIndex { !isName($0) }?.base ?? text.startIndex
        return text[start...]
    }

    /// Names starting with the trailing word, shortest first, at most `limit`; none for a word that is a name already.
    public static func suggestions(for text: String, among names: [String], limit: Int = 6) -> [String] {
        let word = trailingWord(of: text).lowercased()
        guard let first = word.first, first.isLetter, !names.contains(word) else { return [] }
        return names.filter { $0.lowercased().hasPrefix(word) }.sorted { ($0.count, $0) < ($1.count, $1) }.prefix(limit).map { $0 }
    }

    /// `text` with its trailing word replaced by `name`.
    public static func completing(_ text: String, with name: String) -> String {
        String(text.dropLast(trailingWord(of: text).count)) + name
    }

    /// The name among `names` within a third of `unknown`'s length in edits, the nearest first.
    public static func closest(to unknown: String, among names: [String]) -> String? {
        let limit = max(2, unknown.count / 3)
        return names.map { ($0, distance(unknown.lowercased(), $0.lowercased())) }.filter { $0.1 <= limit }
            .min { ($0.1, $0.0) < ($1.1, $1.0) }?.0
    }

    /// The name inside “…” of core's "Unknown token “primery” at 1:1.".
    public static func unknownName(in message: String) -> String? {
        guard message.hasPrefix("Unknown token “"), let end = message.firstIndex(of: "”") else { return nil }  // text: the start of core's error message, parsed and not shown
        return String(message[message.index(message.startIndex, offsetBy: "Unknown token “".count)..<end])  // text: the start of core's error message, parsed and not shown
    }

    private static func distance(_ a: String, _ b: String) -> Int {
        let (a, b) = (Array(a), Array(b))
        var row = Array(0...b.count)
        for (i, x) in a.enumerated() {
            var diagonal = row[0]
            row[0] = i + 1
            for (j, y) in b.enumerated() {
                let above = row[j + 1]
                row[j + 1] = Swift.min(row[j + 1] + 1, row[j] + 1, diagonal + (x == y ? 0 : 1))
                diagonal = above
            }
        }
        return row[b.count]
    }
}
