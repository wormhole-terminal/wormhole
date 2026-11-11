import Foundation
import Testing

@testable import TerminalUI

@MainActor @Suite struct NoticeTests {
    private let t0 = Date(timeIntervalSinceReferenceDate: 1000)
    private func after(_ seconds: Double) -> Date { t0.addingTimeInterval(seconds) }
    private let leave = NoticeTiming.up

    @Test func aCopyShowsForTwoSecondsThenLeavesAndIsRemoved() {
        var rules = NoticeRules()
        #expect(rules.post(.copied(.lines(3)), now: t0) == .show)
        #expect(rules.advance(now: after(1.9), leave: leave) == .ignore)
        #expect(rules.advance(now: after(2), leave: leave) == .hide)
        #expect(rules.advance(now: after(2.1), leave: leave) == .ignore)
        #expect(rules.advance(now: after(2.2), leave: leave) == .remove)
        #expect(rules.phase == .hidden)
    }

    @Test func theSameMessageAgainWithinTwoSecondsIsNotSaidTwice() {
        var rules = NoticeRules()
        _ = rules.post(.copied(.path), now: t0)
        #expect(rules.post(.copied(.path), now: after(1.9)) == .ignore)
        #expect(rules.post(.copied(.path), now: after(2.1)) == .swap, "the chip still shows: the words swap")
        #expect(rules.post(.copied(.command), now: after(2.2)) == .swap)
    }

    @Test func aWarningStaysAndIsNotSaidAgainOnceClosed() {
        var rules = NoticeRules()
        let warning = Notice.warned(.hostFallback)
        #expect(rules.post(warning, now: t0) == .show)
        #expect(rules.advance(now: after(60), leave: leave) == .ignore)
        #expect(rules.clickable)
        #expect(rules.dismiss(now: after(61), leave: leave) == .hide)
        #expect(rules.post(warning, now: after(70)) == .ignore)
    }

    @Test func aCopyOverAWarningGivesWayAndTheWarningComesBack() {
        var rules = NoticeRules()
        _ = rules.post(.warned(.hostFallback), now: t0)
        _ = rules.post(.copied(.command), now: after(1))
        #expect(!rules.clickable)
        #expect(rules.advance(now: after(3), leave: leave) == .restore)
        #expect(rules.clickable)
    }

    @Test func aClickJustAfterAnOfferAppearsChoosesNothing() {
        var rules = NoticeRules()
        _ = rules.post(.warned(.updateReady(version: "1.7.0")), now: t0)
        #expect(!rules.bodyClickActs(at: after(0.499)))
        #expect(rules.bodyClickActs(at: after(0.5)))
    }

    @Test func aCopiedTextIsLinesWhenItHasSeveralAndCharactersOtherwise() {
        #expect(Notice.copied(text: "one\ntwo\nthree\n") == .copied(.lines(3)))
        #expect(Notice.copied(text: "hello") == .copied(.characters(5)))
        #expect(Notice.copied(text: "hello", byProgram: true) == .copied(.programCharacters(5)))
        #expect(Notice.copied(text: "hello", byProgram: true, program: "ssh").words == "ssh copied 5 characters")
        #expect(Notice.copied(text: "a\nb", byProgram: true, program: "ssh").words == "ssh copied 2 lines")
        #expect(Notice.copied(text: "hello", byProgram: true).words == "A program copied 5 characters")
        #expect(Notice.copiedOutput("a\nb") == .copied(.output(lines: 2)))
        #expect(Notice.copiedOutput("") == .copied(.output(lines: 1)))
    }

    @Test func theWordsFollowTheCatalog() {
        #expect(Notice.copied(.lines(3)).words == "Copied 3 lines")
        #expect(Notice.copied(.characters(1)).words == "Copied 1 character")
        #expect(Notice.copied(.output(lines: 7)).words == "Copied output, 7 lines")
        #expect(Notice.driven(.typing(.agent("Claude Code"))).words == "Claude Code is typing")
        #expect(Notice.warned(.updateReady(version: "1.7.0")).words == "Wormhole 1.7.0 is ready — Restart")
        #expect(Notice.failed(.putAtPrompt(.lines)).words == "Not put at the prompt: each line would run in this shell")
    }

    @Test func theCenterShowsPostsAndTakesWarningsDown() {
        var now = t0
        let center = NoticeCenter(now: { now })
        center.post(.copied(.lines(2)))
        #expect(center.shown == .copied(.lines(2)))
        now = after(2.5)
        center.wake()
        #expect(center.leaving)
        now = after(3)
        center.wake()
        #expect(center.shown == nil)
        center.post(.warned(.dataFolder))
        #expect(center.isClickable)
        center.dismiss()
        #expect(center.leaving, "the × closes a warning for good")
    }

    @Test func onlyAHostThatCouldNotStartExplainsItselfAndTheErrorIsKept() {
        let center = NoticeCenter(now: { Date() })
        center.post(.warned(.hostFallback), detail: "no such file: /x/wormhole-host")
        #expect(center.detail == "no such file: /x/wormhole-host")
        center.clickBody()
        #expect(center.infoOpen, "a click on the capsule opens the explanation")
        center.clickInfo()
        #expect(!center.infoOpen, "and the ⓘ closes it again")
        center.dismiss()
        #expect(!center.infoOpen)

        let other = NoticeCenter(now: { Date() })
        other.post(.warned(.moveToApplications))
        other.clickInfo()
        #expect(!other.infoOpen, "the other warnings have nothing more to say")
    }

    @Test func whereTheChipIsOffNothingDropsAndTheBarIsToldInstead() {
        let center = NoticeCenter(now: { Date() })
        var told: [(Notice, String?)] = []
        center.chipOn = { false }
        center.onBar = { told.append(($0, $1)) }
        center.post(.copied(.programCharacters(5)))
        center.post(.warned(.dataFolder), detail: "why")
        #expect(center.shown == nil)
        #expect(told.map(\.0) == [.copied(.programCharacters(5)), .warned(.dataFolder)])
        #expect(told.last?.1 == "why")
    }
}
