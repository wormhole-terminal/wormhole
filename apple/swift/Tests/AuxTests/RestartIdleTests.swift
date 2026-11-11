import Foundation
import Testing

@testable import AuxModel

@Suite struct RestartIdleTests {
    @Test func anAgentThatWorksOrWaitsIsNotIdleButOneAtItsPromptIs() {
        #expect(!RestartIdle.isIdle(.busy, ended: false, commandRunning: true, agentInFront: true))
        #expect(!RestartIdle.isIdle(.attention, ended: false, commandRunning: true, agentInFront: true))
        #expect(!RestartIdle.isIdle(.idle, ended: false, commandRunning: true, agentInFront: false), "npm run dev")
        #expect(RestartIdle.isIdle(.idle, ended: false, commandRunning: true, agentInFront: true), "claude at its prompt")
        #expect(RestartIdle.isIdle(.idle, ended: false, commandRunning: false, agentInFront: false), "a shell at its prompt")
        #expect(RestartIdle.isIdle(.busy, ended: true, commandRunning: true, agentInFront: false), "nothing runs in an ended shell")
    }

    @Test func theAppMustStayIdleWithoutABreak() {
        let start = Date(timeIntervalSince1970: 1_000)
        let at = { (seconds: Double) in start.addingTimeInterval(seconds) }
        var watch = IdleWatch()
        var verdicts: [Bool] = []
        for (idle, seconds) in [(true, 0.0), (true, RestartIdle.idleFor - 1),
            // An agent between two steps: busy for one look, and the wait starts over.
            (false, RestartIdle.idleFor), (true, RestartIdle.idleFor + 1), (true, 2 * RestartIdle.idleFor),
            (true, 2 * RestartIdle.idleFor + 1)]
        {
            verdicts.append(watch.seen(idle: idle, at: at(seconds)))
        }
        #expect(verdicts == [false, false, false, false, false, true], "the wait starts at the first idle look and needs 30 s without a break")
    }
}
