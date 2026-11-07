// Which way a notification goes out (`notifications::route`): the system's centre in a bundled app, a log in a
// test instance (never a real banner), nowhere otherwise. A bundle whose centre can't be had goes nowhere
// rather than to the script GPUI falls back to: the Swift app has no `osascript` route.

import Foundation

public enum NoticeRoute: Sendable, Equatable {
    case center
    /// A test instance: kept in a log (devctl's `notify-log`), never posted.
    case log
    case off

    /// The route for a run: a test instance logs (unless WORMHOLE_NOTIFY names another route), a bundled app
    /// posts through the centre.
    public static func choose(policy: PlatformPolicy, notify: String? = ProcessInfo.processInfo.environment["WORMHOLE_NOTIFY"], bundled: Bool) -> NoticeRoute {
        let logs = notify.map { $0 == "log" } ?? policy.background
        if logs { return .log }
        return bundled ? .center : .off
    }
}
