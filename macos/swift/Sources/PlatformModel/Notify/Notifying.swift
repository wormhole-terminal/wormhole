// Where banners go (`notifications::Notifier`'s routes): the system's notification centre in a bundled app, a log
// in a test instance, nowhere otherwise. A protocol, so a window posts and reads answers without knowing which,
// and a test instance can never raise a real banner.

import Foundation

@MainActor
public protocol Notifying: AnyObject {
    var permission: NoticePermission { get }
    /// A click on a notification, or one of its buttons (Reply's text included), from the system.
    var responses: AsyncStream<NoticeResponse> { get }
    /// Posts `notice`; the same id replaces a terminal's earlier one.
    func post(_ notice: Notice)
    /// Posts `notice` and says what the system answered: throws its error, as the Settings test row shows it.
    func deliver(_ notice: Notice) async throws
    /// Takes down a notification, pending or delivered.
    func remove(id: String)
    /// Asks the system for permission (macOS prompts only while it hasn't been decided), then reads it again.
    func askPermission()
    /// "Needs input" can be delivered as Time Sensitive: signed for it and allowed by the person. Never in a log.
    var deliversTimeSensitive: Bool { get }
}

extension Notifying {
    public var deliversTimeSensitive: Bool { false }
    public func deliver(_ notice: Notice) async throws { post(notice) }
}

/// A test instance's notifier: keeps what would have been posted, as devctl's `notify-log` shows it, and posts
/// nothing. Responses are fed in by hand (`respond`) to drive the reply path.
@MainActor
public final class LogNotifier: Notifying {
    public private(set) var log: [Notice] = []
    public private(set) var permission: NoticePermission = .unavailable
    public let responses: AsyncStream<NoticeResponse>
    private let continuation: AsyncStream<NoticeResponse>.Continuation

    public init() {
        (responses, continuation) = AsyncStream.makeStream()
    }

    public func post(_ notice: Notice) {
        log.removeAll { $0.id == notice.id }
        log.append(notice)
    }

    public func remove(id: String) { log.removeAll { $0.id == id } }
    public func askPermission() {}

    /// What the control folder's `notify-permission` says macOS said, to see the welcome's and Settings' page for each.
    public func pretend(_ permission: NoticePermission) { self.permission = permission }

    /// What a click on the banner would send.
    public func respond(_ response: NoticeResponse) { continuation.yield(response) }
}
