import Foundation
import Testing

/// Agent access (Automation on): what a program outside the app's terminals may do, asked through the app's own command line
/// (`Instance.cli`, a "Scripts" requester: no agent in its chain) and answered through the control folder (`agents`, `agent-ask`,
/// `agent-script`, `agent-grant`), never by a sheet. `control <words…>` asks in-process, without a program behind it.
extension EndToEnd {
    static func access(_ mode: String) -> String {
        #"{"feature_flags": {"unlocked": true, "automation": true}, "agent_access": ""# + mode + #""}"#
    }

    /// Answers each question the client's request puts (Until Quit) until the client has its reply; what `agents` said first.
    @MainActor static func approve(_ client: Client, in app: Instance, with answer: String = "quit") async -> String {
        var first = ""
        for _ in 0..<3 {
            let said = await app.until("agents") { $0.contains("\nasking ") || !client.isRunning }
            if first.isEmpty { first = said }
            guard client.isRunning, said.contains("\nasking ") else { break }
            await app.send(["agent-ask \(answer)"])
        }
        return first
    }

    @Test("asked: a script's request waits for the person; Until Quit lets it in and the next one in without asking")
    @MainActor func accessAsked() async throws {
        try await Instance.with(config: Self.access("ask")) { app in
            await app.send(["agent-script hold"])
            app.expect(await app.reply("agent-ask quit"), contains: "error: no question is up")
            let before = (await app.rows()).live.count
            let client = try app.cli(["new-terminal"])
            let asked = await Self.approve(client, in: app)
            // A program outside any agent is Scripts, whose question offers no "for this session" and no "always".
            app.expect(asked, contains: "Scripts")
            app.expect(asked, contains: "shown deny,quit")
            let status = await client.finish()
            app.expect(status == 0, "the request went through once let in (status \(String(describing: status))):\n\(client.output)")
            await app.until("rows") { Listing($0).live.count == before + 1 }
            app.expect(await app.reply("agents"), contains: "approved memory:")

            let again = try app.cli(["new-terminal"])
            let quiet = await app.until("agents") { $0.contains("\nasking ") || !again.isRunning }
            app.expect(!quiet.contains("\nasking "), "the approval holds for the next request:\n\(quiet)")
            app.expect(await again.finish() == 0, "the second request went through:\n\(again.output)")
            await app.until("rows") { Listing($0).live.count == before + 2 }
        }
    }

    @Test("a question that waits is refused when agent access is turned off in Settings, and Don't Allow refuses at once")
    @MainActor func accessRefused() async throws {
        try await Instance.with(config: Self.access("ask")) { app in
            await app.send(["agent-script hold"])
            let before = (await app.rows()).live.count
            let waiting = try app.cli(["new-terminal"])
            await app.until("agents") { $0.contains("\nasking ") }
            await app.send(["set agent_access=\"off\""])
            await app.until("agents") { !$0.contains("\nasking ") }
            let refused = await waiting.finish()
            app.expect(refused != nil && refused != 0, "the waiting request was refused (status \(String(describing: refused))):\n\(waiting.output)")

            // Ask again, and a script that answers every question with Don't Allow: refused with no question held.
            await app.send(["set agent_access=\"ask\"", "agent-script deny"])
            _ = await app.waitForConfig { $0.range(of: #""agent_access"\s*:\s*"ask""#, options: .regularExpression) != nil }
            let denied = try await app.cliRun(["new-terminal"])
            app.expect(denied.status != nil && denied.status != 0, "Don't Allow refuses:\n\(denied.output)")
            app.expect(!(await app.reply("agents")).contains("\nasking "), "no question is left up")
            app.expect((await app.rows()).live.count == before, "nothing was opened")
        }
    }

    @Test("Settings' mode decides at once: Open lets a request in, Off refuses it; a request that needs nothing is always answered")
    @MainActor func accessModes() async throws {
        try await Instance.with(config: Self.access("off")) { app in
            guard let first = (await app.rows()).live.first else { return }
            app.expect(await app.reply("control list --json"), contains: "{")
            app.expect(await app.reply("control bogus"), contains: "error")
            app.expect(await app.reply("control send-text --terminal \(first) refused-text"), contains: "agent access is off")
            await app.send(["set agent_access=\"open\""])
            // The hook server reads the settings again a moment after a change: asked until it has.
            await app.until("control send-text --terminal \(first) open-text") { !$0.contains("error") && !$0.contains("off") }
            await app.screen(showing: "open-text")
            await app.send(["set agent_access=\"off\""])
            await app.until("control send-text --terminal \(first) closed-again") { $0.contains("agent access is off") }
            app.expect(await app.count("refused-text") == 0, "nothing typed while off")
        }
    }

    @Test("a mode widened by editing config.json outside the app is asked about; Don't Keep writes the earlier mode back")
    @MainActor func accessWidenedOutside() async throws {
        try await Instance.with(config: Self.access("off")) { app in
            guard let first = (await app.rows()).live.first else { return }
            let file = app.scratch.data.appendingPathComponent("config.json")
            try Self.access("open").write(to: file, atomically: true, encoding: .utf8)
            let asked = await app.until("agents") { $0.contains("asking mode") }
            app.expect(asked, contains: "open")
            app.expect(asked, contains: "dontKeep,keep")
            // Until the person answers, the earlier mode is the one in force.
            app.expect(await app.reply("control send-text --terminal \(first) too-early"), contains: "agent access is off")
            await app.send(["agent-ask deny"])
            _ = await app.waitForConfig { $0.range(of: #""agent_access"\s*:\s*"off""#, options: .regularExpression) != nil }
            // The file is written before the desk tells the window the question is settled: the held question goes a moment later.
            let gone = await app.until("agents") { !$0.contains("asking mode") }
            app.expect(!gone.contains("asking mode"), "the question went")

            // Widened again and kept this time: the new mode is in force.
            try Self.access("open").write(to: file, atomically: true, encoding: .utf8)
            await app.until("agents") { $0.contains("asking mode") }
            await app.send(["agent-ask quit"])
            await app.until("control send-text --terminal \(first) kept-open") { !$0.contains("error") && !$0.contains("off") }
            await app.screen(showing: "kept-open")
            app.expect(await app.count("too-early") == 0, "nothing was typed while the question waited")
        }
    }

    @Test("Scripts let in for good by agent-grant are let in without a question after a restart")
    @MainActor func accessGrantedScripts() async throws {
        try await Instance.with(config: Self.access("ask")) { app in
            await app.send(["agent-script hold"])
            app.expect(await app.reply("agent-grant scripts always"), contains: "agent-grant scripts always")
            app.expect(await app.reply("agents"), contains: "approved stored:")
            try await app.restart()
            await app.send(["agent-script hold"])
            app.expect(await app.reply("agents"), contains: "approved stored:")
            let before = (await app.rows()).live.count
            let client = try app.cli(["new-terminal"])
            let quiet = await app.until("agents") { $0.contains("\nasking ") || !client.isRunning }
            app.expect(!quiet.contains("\nasking "), "no question for Scripts after the stored approval:\n\(quiet)")
            app.expect(await client.finish() == 0, "let in:\n\(client.output)")
            await app.until("rows") { Listing($0).live.count == before + 1 }
        }
    }
}
