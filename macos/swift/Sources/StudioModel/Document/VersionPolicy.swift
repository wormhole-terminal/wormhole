// Which saved versions to keep (`theme::saved_versions::keep_policy`): every save of the last hour (at most 120),
// then the newest of each hour for a day, then the newest of each day for 30 days; the original file always.

import Foundation

enum VersionPolicy {
    static let hour = 3600.0, day = 86_400.0, maxRecent = 120, maxAge = 30 * day

    /// For each save (time, pinned), in the same order, whether it stays.
    static func keep(_ saves: [(Date, Bool)], now: Date) -> [Bool] {
        var keep = Array(repeating: false, count: saves.count)
        var recent = 0
        var buckets = Set<[Int]>()
        for index in saves.indices.sorted(by: { saves[$0].0 > saves[$1].0 }) {
            let (at, pinned) = saves[index]
            let age = now.timeIntervalSince(at)
            let seconds = at.timeIntervalSince1970
            if pinned {
                keep[index] = true
            } else if age <= hour {
                recent += 1
                keep[index] = recent <= maxRecent
            } else if age <= day {
                keep[index] = buckets.insert([0, Int((seconds / hour).rounded(.down))]).inserted
            } else if age <= maxAge {
                keep[index] = buckets.insert([1, Int((seconds / day).rounded(.down))]).inserted
            }
        }
        return keep
    }
}
