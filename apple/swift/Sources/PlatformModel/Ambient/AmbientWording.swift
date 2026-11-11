// The short words the ambient surfaces share (`view::sidebar::wording`, `notify::wording`): how long a
// terminal has waited, run or been done, and how long a command took.

import Foundation
import WordsModel

public enum AmbientWording {
    public static func count(_ n: Int, _ one: String, _ many: String) -> String { "\(n) \(n == 1 ? one : many)" }

    /// How long something has taken, short: nil under a minute, then "3m", "1h", "2d".
    public static func took(_ elapsed: TimeInterval) -> String? {
        let minutes = Int(elapsed) / 60
        switch minutes {
        case 0: return nil
        case 1..<60: return "\(minutes)m"
        case 60..<1440: return "\(minutes / 60)h"
        default: return "\(minutes / 1440)d"
        }
    }

    /// How long ago, short: "just now", "12m ago".
    public static func ago(_ elapsed: TimeInterval) -> String {
        took(elapsed).map { Words.text("hosts-heard-ago", ["ago": .text($0)], fallback: "\($0) ago") }
            ?? Words.text("history-just-now", fallback: "just now")
    }

    /// A running command's time on the clock: "0:05", "1:12", "1:02:03".
    public static func clock(_ elapsed: TimeInterval) -> String {
        let secs = Int(elapsed)
        let (hours, minutes, seconds) = (secs / 3600, secs / 60 % 60, secs % 60)
        let tail = String(format: "%02d", seconds)
        return hours > 0 ? "\(hours):" + String(format: "%02d", minutes) + ":" + tail : "\(minutes):" + tail
    }

    /// How long a command took, for a body: "38s", "4m 12s", "1h 5m".
    public static func duration(_ elapsed: TimeInterval) -> String {
        let secs = Int(elapsed)
        switch secs {
        case ..<60: return "\(secs)s"
        case 60..<3600: return "\(secs / 60)m \(secs % 60)s"
        default: return "\(secs / 3600)h \(secs % 3600 / 60)m"
        }
    }

    /// A row's time: how long it has waited ("2m", "now"), run ("4:12"), or been done ("12m ago"); nil when idle.
    public static func elapsed(_ state: AmbientState, since: TimeInterval) -> String? {
        switch state {
        case .needsYou: took(since) ?? "now"
        case .working: clock(since)
        case .done, .failed: ago(since)
        case .idle, .exited: nil
        }
    }
}
