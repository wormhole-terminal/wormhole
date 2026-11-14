import Foundation
import Testing

/// Review notes on a Changes diff (P03): each terminal's own, a line's comment edited and removed, and the review bar's Paste into
/// Terminal and Clear. Read with `review` (`Review.message`, what a paste pastes), the screen and the instance's clipboard.
extension EndToEnd {
    /// The first added line of the open diff, as `diff-spans` numbers its rows, once the rows are cut.
    @MainActor static func addedRow(_ app: Instance) async -> String? {
        let spans = Readout(await app.until("diff-spans 0 40") { Readout($0).all("line").contains { $0.words.dropFirst().first == "added" } })
        return spans.all("line").first(where: { $0.words.dropFirst().first == "added" })?.words.first
    }

    /// A click on that line: the comment dialog is up (a sheet on the window).
    @MainActor static func openComment(_ app: Instance, row: String) async {
        app.expect(await app.reply("comment-at \(row)"), contains: "comment-at a.txt:")
        await app.until("windows") { $0.contains("sheet=true") }
    }

    // GPUI: `features/git/diff/review.rs:212` (a line's comment replaced), `:227` (Remove), `:246` (Paste into Terminal: pasted
    // as text, the comments gone, the diff closed and the keys given back), the comments kept per terminal (`ChangesOf`).
    @Test("a review is its terminal's own; a line's comment is replaced and removed; Paste into Terminal pastes it and empties it")
    @MainActor func reviewPerTerminalEditRemovePaste() async throws {
        try await Instance.with(shell: .zsh) { app in
            let repo = try app.scratch.repository("review-paste-repo", files: ["a.txt": "one\n"])
            try app.scratch.write(repo, "a.txt", "one\ntwo\n")
            guard let a = (await app.terminal(in: repo)).selected else { return }
            await app.until("changes") { $0.contains("phase=loaded") && $0.contains("a.txt") }
            await app.send(["diff 1"])
            guard let row = await Self.addedRow(app) else { return }
            await Self.openComment(app, row: row)
            app.expect(await app.reply("comment first note"), contains: "1 comment")

            // Another terminal on the same repository has a review of its own; back on the first, its comment is there.
            await app.terminal(in: repo)
            await app.until("changes") { $0.contains("phase=loaded") && $0.contains("a.txt") }
            app.expect(await app.reply("review"), contains: "review: none")
            await app.send(["select \(a)"])
            await app.until("review") { $0.contains("first note") }

            // The same line again: the dialog edits its comment, which is replaced, not added to.
            await app.send(["diff 1"])
            guard let again = await Self.addedRow(app) else { return }
            await Self.openComment(app, row: again)
            app.expect(await app.reply("comment second note"), contains: "1 comment")
            let edited = await app.reply("review")
            app.expect(edited.contains("second note") && !edited.contains("first note"), "the comment was replaced:\n\(edited)")

            // Remove takes the line's comment away.
            await Self.openComment(app, row: again)
            app.expect(await app.reply("comment-remove"), contains: "comment-remove: no comments")
            await app.until("windows") { !$0.contains("sheet=true") }
            app.expect(await app.reply("review"), contains: "review: none")

            // zsh's line editor takes a paste as text (bracketed): the review lands at its prompt, unsent.
            await app.until("screen") { $0.contains("%") }
            await Self.openComment(app, row: again)
            app.expect(await app.reply("comment fix this line"), contains: "1 comment")
            app.expect(await app.reply("review-paste"), contains: "copied-instead=false")
            await app.screen(showing: "Review comments on your changes")
            app.expect(await app.reply("review"), contains: "review: none")
            app.expect(await app.until("changes") { $0.contains("open=-") }, contains: "diff=idle")
            app.expect((await app.reply("keys")).hasPrefix("focus=terminal"), "the terminal has the keys after the paste")
            app.expect(await app.count("Review comments on your changes") == 1, "pasted once, not run line by line")
        }
    }

    // GPUI: `features/git/diff/review.rs:254` (no bracketed paste: copied instead, the comments stay), `:275` (Clear).
    @Test("where a paste would run each line, Paste into Terminal copies the review instead and keeps it; Clear empties it")
    @MainActor func reviewCopiedWhereAPasteWouldRun() async throws {
        try await Instance.with { app in
            let repo = try app.scratch.repository("review-copy-repo", files: ["a.txt": "one\n"])
            try app.scratch.write(repo, "a.txt", "one\ntwo\n")
            await app.terminal(in: repo)
            await app.until("changes") { $0.contains("phase=loaded") && $0.contains("a.txt") }
            await app.send(["diff 1"])
            guard let row = await Self.addedRow(app) else { return }
            await Self.openComment(app, row: row)
            app.expect(await app.reply("comment keep it"), contains: "1 comment")
            await app.until("windows") { !$0.contains("sheet=true") }

            // `cat` never asked for bracketed paste: the review goes to the (instance's own) clipboard, not to the program.
            app.expect(await app.reply("review-paste"), contains: "copied-instead=true")
            app.expect(await app.reply("pasteboard"), contains: "Review comments on your changes")
            app.expect(await app.reply("review"), contains: "keep it")
            app.expect(await app.count("Review comments") == 0, "nothing reached the program")

            app.expect(await app.reply("review-clear"), contains: "review-clear")
            app.expect(await app.reply("review"), contains: "review: none")
        }
    }
}
