// How the sheet's sections go into columns (core `sheet::columns`): in order, read down then across, cut so
// the tallest column is as short as it can be.

import Foundation

public enum SheetColumns {
    /// Splits sections of these `heights` into at most `columns` runs.
    public static func split(_ heights: [Int], into columns: Int) -> [Range<Int>] {
        let n = heights.count
        guard n > 0, columns > 0 else { return [] }
        let columns = min(columns, n)
        let prefix = heights.reduce(into: [0]) { $0.append($0.last! + $1) }
        let unreachable = Int.max
        // best[k][i]: the tallest column when the first i sections go into k columns.
        var best = Array(repeating: Array(repeating: unreachable, count: n + 1), count: columns + 1)
        var cut = Array(repeating: Array(repeating: 0, count: n + 1), count: columns + 1)
        best[0][0] = 0
        for k in 1...columns {
            // Fewer sections than columns so far leave nothing to cut.
            for i in k...n {
                for j in (k - 1)..<i where best[k - 1][j] != unreachable {
                    let tallest = max(best[k - 1][j], prefix[i] - prefix[j])
                    if tallest < best[k][i] {
                        best[k][i] = tallest
                        cut[k][i] = j
                    }
                }
            }
        }
        var runs: [Range<Int>] = []
        var end = n
        for k in stride(from: columns, through: 1, by: -1) {
            runs.append(cut[k][end]..<end)
            end = cut[k][end]
        }
        return runs.reversed()
    }
}
