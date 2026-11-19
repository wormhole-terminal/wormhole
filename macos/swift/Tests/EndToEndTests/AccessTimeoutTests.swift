import Foundation
import Testing

/// A question nobody answers: after a minute (`ASK_WAIT_S`, the skill's and the docs' "a minute") it counts as no and is not
/// remembered, whichever window would have held it. Asked through the app's own command line as a script (`Instance.cli`), the
/// question held for the control folder (`agent-script hold`), as `AgentAccessTests` does. The one scenario here waits the minute.
extension EndToEnd {
    @Test("an unanswered question is refused after a minute, and the next request asks again")
    @MainActor func accessQuestionTimesOut() async throws {
        try await Instance.with(config: Self.access("ask")) { app in
            await app.send(["agent-script hold"])
            let before = (await app.rows()).live.count
            let waiting = try app.cli(["new-terminal"])
            await app.until("agents") { $0.contains("\nasking ") }
            // Nobody answers. It is still asked a good way in.
            // In steps well inside one command's own wait.
            for _ in 0..<6 { await app.send(["delay 5000"]) }
            app.expect(await app.reply("agents"), contains: "\nasking ")
            app.expect(waiting.isRunning, "the request still waits after half a minute")

            let status = await waiting.finish(timeout: 60)
            app.expect(status != nil && status != 0, "the request was refused (status \(String(describing: status))):\n\(waiting.output)")
            app.expect(waiting.output.contains("within a minute"), "it says why:\n\(waiting.output)")
            let said = await app.reply("agents")
            app.expect(!said.contains("\nasking "), "no question is left up:\n\(said)")
            app.expect(!said.contains("approved "), "a refusal by time is not remembered:\n\(said)")
            app.expect((await app.rows()).live.count == before, "nothing was opened")

            // Not remembered: the next request is asked again.
            let again = try app.cli(["new-terminal"])
            await app.until("agents") { $0.contains("\nasking ") }
            await app.send(["agent-ask deny"])
            _ = await again.finish()
        }
    }
}
