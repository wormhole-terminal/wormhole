// How long ago a commit reads on the list's right edge: "now", "12m", "3h", "2d", "5w", "4mo", "2y"
// (`wormhole_core::view::panel::age`). A time in the future, from a skewed clock, is "now".

public enum Age {
    public static func text(seconds: Int) -> String {
        let minute = 60, hour = 60 * minute, day = 24 * hour, week = 7 * day
        let year = 31_556_952, month = year / 12
        switch max(seconds, 0) {
        case ..<minute: return "now"
        case ..<hour: return "\(seconds / minute)m"
        case ..<day: return "\(seconds / hour)h"
        case ..<week: return "\(seconds / day)d"
        case ..<month: return "\(seconds / week)w"
        case ..<year: return "\(seconds / month)mo"
        default: return "\(seconds / year)y"
        }
    }
}
