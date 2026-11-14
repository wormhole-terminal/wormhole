import Foundation
import Testing

/// What a program says about itself, in the banners' and the row's words: a kitty notification (OSC 99) sent in pieces, a
/// long command's finish and failure as the core words them (`notify::wording`), and the Program Status Protocol (OSC 7501:
/// blocked, done, error) on the row, in the status bar and in the banner. The banners are the instance's log of what it
/// would post (`notice-log`: title and body).
extension EndToEnd {
    /// The logged banners' `title=… body=…` parts, oldest first.
    private static func banners(_ log: String) -> [(title: String, body: String)] {
        log.split(separator: "\n").compactMap { line in
            guard let title = line.range(of: " title="), let body = line.range(of: " body=", range: title.upperBound..<line.endIndex) else { return nil }
            return (String(line[title.upperBound..<body.lowerBound]), String(line[body.upperBound...]))
        }
    }

    @Test("a kitty notification sent in pieces is one banner; a long command's finish names it and how long it ran, its failure the exit code and its last line")
    @MainActor func kittyNoticeAndCommandBannerWords() async throws {
        try await Instance.with(shell: .zsh) { app in
            await app.markPrompts()
            // OSC 99: the title in two chunks (d=0: more follows), then the body, which completes it.
            await app.shell(#"printf '\e]99;i=1:d=0;Ninety\a\e]99;i=1:d=0;-nine\a\e]99;i=1:p=body;kitty-body\a'"#)
            let kitty = Self.banners(await app.until("notice-log") { $0.contains("kitty-body") })
            app.expect(kitty.contains { $0.title == "Ninety-nine" && $0.body == "kitty-body" }, "one banner with the joined title and the body: \(kitty)")
            // (From the third banner within ten seconds the core posts a summary instead: the next two come a window later.)
            await app.send(["delay 10500"], timeout: 30)

            await app.send(["set notify_command_after=5", "set notify_command_done=true", "set notify_command_failed=true"])
            await app.until("get notify_command_after") { $0.contains("5") }
            await app.shell("/bin/sleep 6")
            let done = Self.banners(await app.until("notice-log", timeout: 30) { $0.contains("sleep 6 finished") })
            // The title is the command as the row names it (the program without its folder) and "finished".
            guard let finished = done.first(where: { $0.title == "sleep 6 finished" }) else { return app.fail("a banner titled with the command and finished: \(done)") }
            app.expect(finished.body.range(of: #"^\d+s$"#, options: .regularExpression) != nil, "its body is how long it ran: \(finished.body)")

            // A failure: the exit code in the title, how long it ran and the last line it printed in the body.
            await app.shell("/bin/sh -c 'echo last-words-of-it; sleep 6; exit 3'")
            let failed = Self.banners(await app.until("notice-log", timeout: 30) { $0.contains("exit 3") })
            guard let failure = failed.first(where: { $0.title.hasSuffix("failed · exit 3") }) else { return app.fail("a banner that says failed and the exit code: \(failed)") }
            app.expect(failure.body.range(of: #"^after \d+s"#, options: .regularExpression) != nil, "its body says after how long: \(failure.body)")
            app.expect(failure.body, contains: "last-words-of-it")
        }
    }

    @Test("a program's status reports (OSC 7501): blocked is needs input with its question, done is finished with its last words, error is failed")
    @MainActor func programStatusWords() async throws {
        try await Instance.with(shell: .zsh) { app in
            guard let terminal = (await app.rows()).live.first else { return app.fail("a terminal") }
            // The program reports and runs on (a record ends with its program): blocked on a permission, with its question.
            await app.shell(#"printf '\e]7501;state=blocked:kind=permission:app=terraform:msg=QXBwbHkgMyB0byBhZGQ/\e\\'; /bin/sleep 120"#)
            await app.until("rows") { Listing($0).row(terminal)?.activity == "needsInput" }
            let asked = Self.banners(await app.until("notice-log") { $0.contains("Apply 3 to add?") })
            app.expect(asked.contains { $0.title.hasSuffix("needs input") && $0.body == "Apply 3 to add?" }, "the banner asks the program's question: \(asked)")
            let bar = Readout(await app.until("status-items") { Readout($0).first("attention") != nil })
            app.expect(bar.first("attention")?.words.first == String(terminal), "the status bar's button is for this terminal:\n\(bar.text)")
            app.expect(bar.first("attention")?["words"]?.hasPrefix("needs input") == true, "in the bar's words:\n\(bar.text)")

            // Done, with its last words leading the banner.
            await app.send(["type \u{3}"])
            await app.shell(#"printf '\e]7501;state=done:msg=VXBncmFkZWQgMTIgcGFja2FnZXM=\e\\'; /bin/sleep 120"#)
            await app.until("rows") { Listing($0).row(terminal)?.activity == "finished" }
            let done = Self.banners(await app.until("notice-log") { $0.contains("Upgraded 12 packages") })
            app.expect(done.contains { $0.title.hasSuffix("finished") && $0.body.hasPrefix("Upgraded 12 packages") }, "the banner leads with the program's words: \(done)")
            await app.send(["delay 10500"], timeout: 30)

            // Error: the row is failed.
            await app.send(["type \u{3}"])
            await app.shell(#"printf '\e]7501;state=error:msg=cnN5bmMgZmFpbGVk\e\\'; /bin/sleep 120"#)
            await app.until("rows") { Listing($0).row(terminal)?.activity == "failed" }
            let failed = Self.banners(await app.until("notice-log") { $0.contains("rsync failed") })
            app.expect(failed.contains { $0.title.contains("failed") && $0.body.contains("rsync failed") }, "the banner says what failed: \(failed)")
            await app.send(["type \u{3}"])
        }
    }
}
