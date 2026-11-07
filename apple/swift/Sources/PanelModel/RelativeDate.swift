// "just now", "5 min ago", "3 d ago": the History tab's dates. `now` is a parameter so tests need no clock.

import Foundation

public enum RelativeDate {
    public static func text(_ date: Date, now: Date) -> String {
        let seconds = Int(now.timeIntervalSince(date))
        switch seconds {
        case ..<60: return "just now"
        case ..<3_600: return "\(seconds / 60) min ago"
        case ..<86_400: return "\(seconds / 3_600) h ago"
        case ..<(86_400 * 30): return "\(seconds / 86_400) d ago"
        case ..<(86_400 * 365): return "\(seconds / (86_400 * 30)) mo ago"
        default: return "\(seconds / (86_400 * 365)) y ago"
        }
    }
}
