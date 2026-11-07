// What a banner is, as values (`platform::notifications` and `notify`): the kind of event, how hard it
// interrupts, the buttons it carries and what a click on it answers. The words and the burst rules are core's
// (`notify::text`, `Bursts`); this is what reaches the notification centre.

import Foundation

/// What happened in a terminal that may be worth telling (`notify::Kind`).
public enum NoticeKind: Sendable, Equatable {
    case needsYou, agentDone, commandDone, commandFailed, agentFailed, bell, notice
}

/// How hard a banner interrupts (`UNNotificationInterruptionLevel`).
public enum NoticeLevel: Sendable, Equatable {
    /// Notification Center only: no banner, no sound.
    case passive
    case active
    /// Breaks through a Focus that allows it.
    case timeSensitive
}

/// Which buttons a notification carries (`platform::notifications::Category`).
public enum NoticeCategory: String, Sendable, CaseIterable {
    /// Open, Reply…
    case needsYou = "wormhole.needs-you"
    /// Open, Show Changes.
    case finished = "wormhole.finished"
    /// No buttons; a click opens.
    case plain
}

/// A button of a notification: what it answers with, its label, and what pressing it does.
public struct NoticeButton: Sendable, Equatable {
    public enum Kind: Sendable, Equatable {
        /// Brings the app forward.
        case foreground
        /// A text field: what is typed arrives as the response's reply, the app left where it is.
        case reply
    }

    public var id: String
    public var label: String
    public var kind: Kind

    public static let open = NoticeButton(id: "open", label: "Open", kind: .foreground)
    public static let reply = NoticeButton(id: "reply", label: "Reply…", kind: .reply)
    public static let showChanges = NoticeButton(id: "show-changes", label: "Show Changes", kind: .foreground)

    public static func buttons(for category: NoticeCategory) -> [NoticeButton] {
        switch category {
        case .needsYou: [.open, .reply]
        case .finished: [.open, .showChanges]
        case .plain: []
        }
    }
}

/// The level a kind is delivered at: needs input is Time Sensitive when the build is entitled to it, else Active;
/// an agent finishing, a failure and a program's own notification are Active; a command finishing and the bell
/// are Passive (`notify::interruption_level`).
public func noticeLevel(_ kind: NoticeKind, hasTimeSensitive: Bool) -> NoticeLevel {
    switch kind {
    case .needsYou where hasTimeSensitive: .timeSensitive
    case .needsYou, .agentDone, .commandFailed, .agentFailed, .notice: .active
    case .commandDone, .bell: .passive
    }
}

/// One notification, ready for any route.
public struct Notice: Sendable, Equatable {
    public var id: String
    /// One thread per terminal, so the centre stacks its banners.
    public var thread: String
    public var title: String
    public var subtitle: String
    public var body: String
    public var level: NoticeLevel
    public var sound: Bool
    public var category: NoticeCategory

    public init(
        id: String, thread: String, title: String, subtitle: String = "", body: String, level: NoticeLevel = .active,
        sound: Bool = false, category: NoticeCategory = .plain
    ) {
        (self.id, self.thread, self.title, self.subtitle, self.body) = (id, thread, title, subtitle, body)
        (self.level, self.sound, self.category) = (level, sound, category)
    }
}

/// A click on one of our notifications, or one of its buttons.
public struct NoticeResponse: Sendable, Equatable {
    public var id: String
    /// nil for a click on the notification itself.
    public var action: String?
    /// What was typed into Reply.
    public var reply: String?
    public init(id: String, action: String? = nil, reply: String? = nil) { (self.id, self.action, self.reply) = (id, action, reply) }
}

/// Whether the system lets Wormhole notify.
public enum NoticePermission: Sendable, Equatable {
    case checking
    /// The system hasn't asked yet; nothing is shown until it has.
    case notAsked
    case allowed(timeSensitive: Bool)
    /// Off in System Settings.
    case denied
    /// Not an app bundle (`swift run`), or a test instance: no centre.
    case unavailable
}
