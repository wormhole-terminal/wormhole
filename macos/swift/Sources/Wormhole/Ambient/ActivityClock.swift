// When each terminal's activity last changed, for the words that count from it ("working 3m"). The core's rows carry the
// finished words for the sidebar but not the instant, so the app notes the first time it sees an activity.

import Foundation
import WorkspaceModel

@MainActor
final class ActivityClock {
    private var seen: [TerminalID: (activity: Activity, since: Date)] = [:]

    func since(_ id: TerminalID, _ activity: Activity, now: Date = Date()) -> Date {
        if let known = seen[id], known.activity == activity { return known.since }
        seen[id] = (activity, now)
        return now
    }
}
