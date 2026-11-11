// Runs the notification rules for the app: a look at the terminals goes to the rules (`NoticeDeciding`), and what
// they say goes where it is delivered, the banners to `Notifying`, the bounce and VoiceOver's words to the hooks
// the app gives. A click on a banner comes back the other way. It reads no clock of its own state: the two clocks
// are given, so a test drives time.

import Foundation

@MainActor
public final class NoticeCoordinator {
    private let rules: any NoticeDeciding
    private let notifier: any Notifying
    private let monotonicMs: @Sendable () -> UInt64
    private let wallMs: @Sendable () -> UInt64
    /// The Dock bounces once, informationally.
    public var onBounce: () -> Void = {}
    /// VoiceOver says it (`urgent` interrupts).
    public var onSay: (String, Bool) -> Void = { _, _ in }
    /// What a click on a banner asks of the app.
    public var onOutcome: (NoticeOutcome) -> Void = { _ in }
    private var listening: Task<Void, Never>?

    public init(
        rules: any NoticeDeciding, notifier: any Notifying,
        monotonicMs: @escaping @Sendable () -> UInt64 = { UInt64(ProcessInfo.processInfo.systemUptime * 1000) },
        wallMs: @escaping @Sendable () -> UInt64 = { UInt64(Date().timeIntervalSince1970 * 1000) }
    ) {
        (self.rules, self.notifier, self.monotonicMs, self.wallMs) = (rules, notifier, monotonicMs, wallMs)
    }

    /// Starts hearing clicks on banners. Once.
    public func listen() {
        guard listening == nil else { return }
        listening = Task { [weak self, responses = notifier.responses] in
            for await response in responses { self?.handle(response) }
        }
    }

    /// A look at every terminal of every window: after an event that changed one (a hook, a mark, a bell, the
    /// window's focus), never on a timer.
    public func look(
        _ tabs: [NoticeTab], settings: NoticeSettings = NoticeSettings(), appActive: Bool, timeSensitive: Bool = false
    ) {
        let actions = rules.observe(
            tabs, settings: settings, appActive: appActive, timeSensitive: timeSensitive, nowMs: monotonicMs(),
            wallMs: wallMs())
        for action in actions {
            switch action {
            case .post(let notice): notifier.post(notice)
            case .retract(let id): notifier.remove(id: id)
            case .bounce: onBounce()
            case .say(let text, let urgent): onSay(text, urgent)
            }
        }
    }

    /// A terminal closed or came to the front: its banner goes.
    public func dismiss(tab: Int64) {
        if let id = rules.dismiss(tab: tab) { notifier.remove(id: id) }
    }

    /// "Send test": posts the banner that needs input would be.
    public func sendTest(name: String, settings: NoticeSettings = NoticeSettings(), timeSensitive: Bool = false) {
        notifier.post(rules.testNotice(name: name, settings: settings, timeSensitive: timeSensitive))
    }

    func handle(_ response: NoticeResponse) {
        let (outcome, retract) = rules.respond(response)
        if let retract { notifier.remove(id: retract) }
        onOutcome(outcome)
    }
}
