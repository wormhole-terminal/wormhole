// When a save was made, in words that need no clock: "just now", "12 minutes ago", "3 hours ago", then the day
// (`studio/versions.rs` `when`).

import Foundation
import WordsModel

enum VersionTime {
    static func words(for saved: Date, now: Date) -> String {
        let seconds = max(0, Int(now.timeIntervalSince(saved)))
        switch seconds {
        case 0..<60: return Words.text("studio-versions-just-now", fallback: "just now")
        case 60..<3_600: return plural(seconds / 60, "minute")
        case 3_600..<172_800: return plural(seconds / 3_600, "hour")
        default: return saved.formatted(.dateTime.day().month(.abbreviated).year())
        }
    }

    private static func plural(_ count: Int, _ unit: String) -> String { "\(count) \(unit)\(count == 1 ? "" : "s") ago" }
}
