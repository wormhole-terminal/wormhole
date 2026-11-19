// Helping the person who typed a name that doesn't exist: the closest one that does ("Did you mean `primary`?"), as
// core's `suggest::closest` does. The names offered while typing are core's (`StudioFiles.completions`).

import Foundation

public enum NameCompletion {
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
