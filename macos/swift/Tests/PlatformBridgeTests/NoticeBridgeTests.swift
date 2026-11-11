import Foundation
import PlatformModel
import Testing

@testable import PlatformBridge

/// The notification rules of core, driven the way the app does: a look at the terminals, the clock given, the
/// banners going to a log (never to the real notification centre).
@MainActor @Suite struct NoticeBridgeTests {
    private let log = LogNotifier()
    private let coordinator: NoticeCoordinator
    private let clock = Clock()
    private let said = Said()

    final class Clock: @unchecked Sendable {
        var ms: UInt64 = 0
    }

    @MainActor final class Said {
        var bounces = 0
        var words: [(String, Bool)] = []
        /// What clicks asked of the app, heard as they arrive.
        let outcomes: AsyncStream<NoticeOutcome>
        let outcome: AsyncStream<NoticeOutcome>.Continuation

        init() { (outcomes, outcome) = AsyncStream.makeStream() }
    }

    init() {
        let clock = clock
        coordinator = NoticeCoordinator(
            rules: PlatformBridge.makeNoticeRules(), notifier: log, monotonicMs: { clock.ms },
            wallMs: { 1_700_000_000_000 + clock.ms })
        let said = said
        coordinator.onBounce = { said.bounces += 1 }
        coordinator.onSay = { said.words.append(($0, $1)) }
        coordinator.onOutcome = { said.outcome.yield($0) }
    }

    private func agent(_ id: Int64, _ state: NoticeState, since: UInt64 = 0) -> NoticeTab {
        var tab = NoticeTab(session: id, tab: id, name: "terminal \(id)", state: state, sinceMs: since)
        (tab.source, tab.who) = (.agent, "Claude Code")
        return tab
    }

    private func look(_ tabs: [NoticeTab], at seconds: UInt64, active: Bool = false, timeSensitive: Bool = false) {
        clock.ms = seconds * 1000
        coordinator.look(tabs, settings: NoticeSettings(), appActive: active, timeSensitive: timeSensitive)
    }

    @Test func anAgentThatNeedsYouIsABannerWithItsQuestionAndThenItIsTakenDown() {
        look([agent(7, .working)], at: 0)
        var asking = agent(7, .needsYou, since: 5000)
        asking.message = "Approve cargo test?"
        look([asking], at: 5, timeSensitive: true)
        #expect(log.log.count == 1)
        let banner = log.log[0]
        #expect(banner.id == "wormhole/tab/7" && banner.thread == "wormhole/terminal/7")
        #expect(banner.title == "terminal 7 needs input" && banner.body == "Approve cargo test?")
        #expect(banner.level == .timeSensitive && banner.category == .needsYou && banner.sound)
        #expect(said.words.first?.1 == true, "VoiceOver says it, urgently")
        look([agent(7, .working)], at: 6)
        #expect(log.log.isEmpty, "the agent moved on: the banner says nothing true any more")
    }

    @Test func threeFinishingTogetherBecomeOneSummary() {
        look((1...3).map { agent($0, .working) }, at: 0)
        look([agent(1, .done), agent(2, .working), agent(3, .working)], at: 2)
        look([agent(1, .done), agent(2, .done), agent(3, .working)], at: 4)
        #expect(log.log.map(\.id) == ["wormhole/tab/1", "wormhole/tab/2"])
        look([agent(1, .done), agent(2, .done), agent(3, .done)], at: 6)
        #expect(log.log.map(\.id) == ["wormhole/summary"])
        #expect(log.log[0].title == "3 terminals finished")
    }

    @Test func theDockBouncesOnceWhileAway() {
        look([agent(1, .working)], at: 0)
        look([agent(1, .needsYou)], at: 1)
        #expect(said.bounces == 1)
    }

    @Test func lookingAtTheTerminalTakesItsBannerDown() {
        look([agent(2, .working)], at: 0)
        look([agent(2, .needsYou)], at: 1)
        #expect(log.log.count == 1)
        var seen = agent(2, .needsYou)
        (seen.selected, seen.key) = (true, true)
        look([seen], at: 2, active: true)
        #expect(log.log.isEmpty)
    }

    @Test(.timeLimit(.minutes(1))) func aReplyFromTheBannerTypesIntoTheTerminalAndTakesTheBannerDown() async {
        look([agent(9, .working)], at: 0)
        look([agent(9, .needsYou)], at: 1)
        coordinator.listen()
        log.respond(NoticeResponse(id: "wormhole/tab/9", action: "reply", reply: "yes"))
        var heard = said.outcomes.makeAsyncIterator()
        #expect(await heard.next() == .reply(tab: 9, text: "yes"))
        #expect(log.log.isEmpty, "answered: the banner is down")
        log.respond(NoticeResponse(id: "wormhole/summary"))
        #expect(await heard.next() == .activate)
    }

    @Test func theTestBannerIsAsNeedsInputWouldBe() {
        coordinator.sendTest(name: "api", timeSensitive: true)
        #expect(log.log.first?.title == "api needs input")
        #expect(log.log.first?.level == .timeSensitive)
    }

    @Test func aTerminalClosingTakesItsBannerDown() {
        look([agent(4, .working)], at: 0)
        look([agent(4, .needsYou)], at: 1)
        coordinator.dismiss(tab: 4)
        #expect(log.log.isEmpty)
        coordinator.dismiss(tab: 4)
    }

    @Test func swiftsDefaultsAreCoresDefaults() {
        #expect(NoticeSettings() == PlatformBridge.defaultNoticeSettings())
    }
}
