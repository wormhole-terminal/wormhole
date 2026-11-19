import Foundation
import Testing

/// A banner answered: the reply typed into a needs-input banner goes to the terminal it is about, and Open selects that
/// terminal. The banner is the one the agent's own hook caused (not `notify-sim`), the answer what the notification centre
/// hands the app (`notice-respond`: the same call its delegate makes).
extension EndToEnd {
    @Test("a hook's needs-input banner: a reply typed into it is typed into that terminal with Return, and Open selects it")
    @MainActor func bannerReplyAndOpen() async throws {
        try await Instance.with(shell: .agent("codex")) { app in
            await app.send(["key cmd-t"])
            let two = Listing(await app.until("rows") { Listing($0).live.count == 2 })
            guard let waiting = two.live.first, let other = two.live.last else { return }
            await app.send(["select \(other)"])
            // Once the app has seen that an agent runs there (a hook that comes before is undone when it does).
            await app.until("notice-tabs") { $0.contains("\"\(waiting):idle agent") }
            try await app.report(agent: "codex", event: "PermissionRequest", terminal: waiting, becomes: "needsInput")
            // The hook's banner: about that terminal, saying it needs input.
            let log = await app.until("notice-log") { $0.contains("needs input") }
            guard let line = log.split(separator: "\n").last(where: { $0.contains("needs input") }), let id = line.split(separator: " ").first.map(String.init) else {
                return app.fail("a needs-input banner:\n\(log)")
            }
            app.expect(line.contains("category="), "the banner carries its category (what its buttons are): \(line)")

            // The reply goes to the terminal that asked, not to the selected one.
            await app.send(["notice-respond \(id) reply yes-go-ahead"])
            await app.send(["select \(waiting)"])
            await app.screen(showing: "yes-go-ahead")
            await app.send(["select \(other)"])
            app.expect(!(await app.reply("screen")).contains("yes-go-ahead"), "the selected terminal got nothing")

            // The banner opened (a click on it): its terminal is selected.
            app.expect((await app.rows()).selected == other, "the other terminal is selected before")
            await app.send(["notice-respond \(id) open"])
            await app.until("rows") { Listing($0).selected == waiting }
        }
    }
}
