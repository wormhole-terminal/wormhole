import Foundation
import Testing

/// Reply typed as GPUI types it (PL03, `Workspace::type_reply`): trimmed, sent as a bracketed paste, Return pressed after it. Every
/// other reply scenario runs against `cat`, which cannot tell a paste from typing; a real line editor can. Here zsh's own
/// `bracketed-paste` widget is replaced by one that notes what it was given, so the text must have arrived between the paste
/// brackets, and the command it holds must have run, which only the Return after it does.
extension EndToEnd {
    @Test("a reply reaches a line editor as one bracketed paste and Return follows it, trimmed")
    @MainActor func replyIsAPasteThenReturn() async throws {
        try await Instance.with(shell: .zsh) { app in
            let seen = app.scratch.home.appendingPathComponent("paste-seen.txt")
            try app.scratch.write(
                app.scratch.home, "spy.zsh",
                "paste-spy() { zle .bracketed-paste PASTED; print -r -- \"paste:${PASTED}:\" >> \(seen.path); LBUFFER+=$PASTED }\nzle -N bracketed-paste paste-spy\n")
            await app.shell("source \(app.scratch.home.appendingPathComponent("spy.zsh").path)")
            await app.shell("print ready-$((1+1))")
            _ = await app.screen(showing: "ready-2")
            guard let terminal = (await app.rows()).selected else { return app.fail("a selected terminal") }

            // Spaces around it are the reply's to lose: the shell is given the command and nothing else.
            _ = await app.reply("ambient-run reply \(terminal)    print reply-ran-$((6*7))   ")
            let screen = await app.screen(showing: "reply-ran-42")
            app.expect(screen.contains("reply-ran-42"), "Return followed the reply, so it ran:\n\(screen)")
            let noted = (try? String(contentsOf: seen, encoding: .utf8)) ?? ""
            app.expect(noted == "paste:print reply-ran-$((6*7)):\n", "the text arrived as one trimmed paste, got: \(noted.debugDescription)")
        }
    }
}
