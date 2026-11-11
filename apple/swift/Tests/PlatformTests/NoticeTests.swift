import Foundation
import PlatformModel
import Testing

@Suite struct NoticeTests {
    @Test func theLevelAKindIsDeliveredAt() {
        #expect(noticeLevel(.needsYou, hasTimeSensitive: true) == .timeSensitive)
        #expect(noticeLevel(.needsYou, hasTimeSensitive: false) == .active)
        for kind in [NoticeKind.agentDone, .commandFailed, .agentFailed, .notice] { #expect(noticeLevel(kind, hasTimeSensitive: true) == .active) }
        for kind in [NoticeKind.commandDone, .bell] { #expect(noticeLevel(kind, hasTimeSensitive: true) == .passive) }
    }

    @Test func theButtonsOfEachCategory() {
        #expect(NoticeButton.buttons(for: .needsYou).map(\.label) == ["Open", "Reply…"])
        #expect(NoticeButton.buttons(for: .needsYou).map(\.kind) == [.foreground, .reply])
        #expect(NoticeButton.buttons(for: .finished).map(\.label) == ["Open", "Show Changes"])
        #expect(NoticeButton.buttons(for: .plain).isEmpty)
    }

    @Test func aTestInstanceLogsAndABundleUsesTheCentre() {
        let background = PlatformPolicy(background: true), normal = PlatformPolicy(background: false)
        #expect(NoticeRoute.choose(policy: background, notify: nil, bundled: true) == .log)
        #expect(NoticeRoute.choose(policy: normal, notify: nil, bundled: true) == .center)
        #expect(NoticeRoute.choose(policy: normal, notify: nil, bundled: false) == .off)
        #expect(NoticeRoute.choose(policy: normal, notify: "log", bundled: true) == .log)
        #expect(NoticeRoute.choose(policy: background, notify: "center", bundled: true) == .center, "WORMHOLE_NOTIFY names another route")
    }

    @Test @MainActor func theLogKeepsWhatWouldHaveBeenPostedAndReplacesByID() async {
        let log = LogNotifier()
        log.post(Notice(id: "tab-1", thread: "t1", title: "diff perf needs you", body: "Approve?", level: .timeSensitive, category: .needsYou))
        log.post(Notice(id: "tab-1", thread: "t1", title: "diff perf finished", body: "Done", category: .finished))
        log.post(Notice(id: "tab-2", thread: "t2", title: "build", body: "ok"))
        #expect(log.log.map(\.title) == ["diff perf finished", "build"])
        log.remove(id: "tab-2")
        #expect(log.log.count == 1)
        log.respond(NoticeResponse(id: "tab-1", action: "reply", reply: "yes"))
        var iterator = log.responses.makeAsyncIterator()
        #expect(await iterator.next() == NoticeResponse(id: "tab-1", action: "reply", reply: "yes"))
    }

    @Test func ownerAuthenticationErrorsMap() {
        let domain = AuthErrorMapping.domain
        #expect(AuthErrorMapping.map(domain: domain, code: -2, message: "x") == .cancelled)
        #expect(AuthErrorMapping.map(domain: domain, code: -9, message: "x") == .cancelled)
        #expect(AuthErrorMapping.map(domain: domain, code: -1, message: "no") == .failed("no"))
        #expect(AuthErrorMapping.map(domain: domain, code: -5, message: "no passcode") == .unavailable("no passcode"))
        #expect(AuthErrorMapping.map(domain: domain, code: -999, message: "?") == .failed("LAError -999: ?"))
        #expect(AuthErrorMapping.map(domain: "other", code: 3, message: "m") == .failed("other (3): m"))
    }

    @Test func aTestInstanceNeverRaisesThePrompt() async throws {
        let scripted = ScriptedAuthenticator(environment: [:])
        await #expect(throws: AuthError.cancelled) { try await scripted.authenticate(reason: "trust") }
        await #expect(throws: AuthError.failed("denied by WORMHOLE_AUTH")) {
            try await ScriptedAuthenticator(environment: ["WORMHOLE_AUTH": "deny"]).authenticate(reason: "x")
        }
        try await ScriptedAuthenticator(environment: ["WORMHOLE_AUTH": "allow"]).authenticate(reason: "x")
    }
}
