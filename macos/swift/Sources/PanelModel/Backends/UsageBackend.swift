// Where the Usage tab's reports come from: what the agents' status lines and hooks forwarded to the app
// (`wormhole_core::agents::usage`). One value for every window.

import Foundation

public protocol UsageBackend: Sendable {
    func reports() async -> [UsageReport]
    /// Fires when an agent reported anew.
    func changes() -> AsyncStream<Void>
    /// How long until a word the reports show reads differently from `date` (the core's rule: each second in a limit's last
    /// minute, else each minute); nil when none moves.
    func nextChange(from date: Date) -> Duration?
}

extension UsageBackend {
    /// Without the core's rule, the words move once a minute.
    public func nextChange(from date: Date) -> Duration? { .seconds(60 - Int(date.timeIntervalSince1970) % 60) }
}
