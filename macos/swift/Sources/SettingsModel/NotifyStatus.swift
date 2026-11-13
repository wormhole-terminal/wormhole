// What the Notifications page shows about macOS: whether banners are allowed, and what came of the last test. The
// app fills it (`SettingsModel.notifyStatus`); the page only draws it. The states are `notifications::Permission`
// and `Test` of the GPUI app.

public struct NotifyStatus: Sendable, Equatable {
    public enum Permission: Sendable, Equatable {
        case checking, notAsked, denied, outsideBundle
        case allowed(timeSensitive: Bool)
    }

    public enum Test: Sendable, Equatable {
        case idle, waiting, sent, denied, unanswered
        case refused(String)
    }

    public var permission = Permission.checking
    public var test = Test.idle
    /// This build can deliver "needs input" as Time Sensitive (signed for it, and allowed by the person).
    public var timeSensitive = false

    public init() {}
}
