// Who hears about a terminal, and how (`notify`): the rules are core's, behind this protocol. A look at the
// terminals goes in with the clock; what the person is owed comes out. Nothing here posts anything: delivery is
// `Notifying`'s (the notification centre, a log), the Dock's and VoiceOver's.

import Foundation

/// What the person's settings say about being told (the fields of core's `Settings` the rules read).
public struct NoticeSettings: Sendable, Equatable {
    public enum Sound: Sendable, Equatable { case needsYou, all, none }

    public var notifications = true
    public var needsYou = true
    public var agentDone = true
    public var commandDone = true
    public var commandFailed = true
    /// A command is told only past this many seconds.
    public var commandAfterSecs: UInt32 = 10
    public var bell = false
    public var onlyUnseen = true
    /// A banner while Wormhole is in front, else nothing.
    public var bannerInFront = false
    public var sound = Sound.needsYou
    public var groupBursts = true
    public var dockBounce = true

    public init() {}
}

/// One thing the person's attention is owed, in the order it happened.
public enum NoticeAction: Sendable, Equatable {
    case post(Notice)
    case retract(id: String)
    /// One informational Dock bounce.
    case bounce
    /// Something for VoiceOver to say; `urgent` interrupts what it is saying.
    case say(String, urgent: Bool)
}

/// What a click on a banner, or one of its buttons, asks of the app.
public enum NoticeOutcome: Sendable, Equatable {
    case nothing
    /// Bring Wormhole forward (the summary, the test banner).
    case activate
    case open(tab: Int64)
    case showChanges(tab: Int64)
    /// Type `text` into the terminal and press Return, without bringing anything forward.
    case reply(tab: Int64, text: String)
}

public protocol NoticeDeciding: Sendable {
    /// Read every terminal and say what changed since the last look. `settings` are the look's own, for a
    /// terminal in a profile that names none; `timeSensitive`: the build is entitled and the person allows it.
    func observe(
        _ tabs: [NoticeTab], settings: NoticeSettings, appActive: Bool, timeSensitive: Bool, nowMs: UInt64,
        wallMs: UInt64
    ) -> [NoticeAction]
    /// A terminal's banner goes (it closed, or the person looked at it): the id to take down, if one is up.
    func dismiss(tab: Int64) -> String?
    /// What a click asks of the app, and the banner to take down when the click did not already.
    func respond(_ response: NoticeResponse) -> (outcome: NoticeOutcome, retract: String?)
    /// "Send test": a banner as "needs input" would be.
    func testNotice(name: String, settings: NoticeSettings, timeSensitive: Bool) -> Notice
}
