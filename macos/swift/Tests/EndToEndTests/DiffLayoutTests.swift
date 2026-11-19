import Foundation
import Testing

/// How the diff view lays a diff out (P03): old against new in two columns where there is room, one column otherwise, and a
/// binary file's card. Read with `diff-drawn` (what `DiffView` decided from its own width) and `diff-header`.
extension EndToEnd {
    /// The open diff's layout line, once the view has laid it out as `kind`.
    @MainActor static func drawn(_ app: Instance, _ kind: String, timeout: TimeInterval = 20) async -> Readout {
        Readout(await app.until("diff-drawn", timeout: timeout) { Readout($0).first("diff-drawn")?.words.first == kind })
    }

    // GPUI: `features/git/diff/mod.rs:175` (split = the setting where it fits), `:289` (the header's switch writes `diff_split`),
    // core `view::limits::diff_split_fits` (560 pt), `git::split_rows` (a removed run against an added one, line by line).
    @Test("a diff is split where the terminal has room and the setting asks; the header's switch and the setting both change it")
    @MainActor func diffSplitAndUnified() async throws {
        try await Instance.with { app in
            let repo = try app.scratch.repository("split-repo", files: ["five.txt": "a\nb\nc\nd\ne\n"])
            try app.scratch.write(repo, "five.txt", "a\nB\nC\nD\ne\n")
            await app.terminal(in: repo)
            await app.until("changes") { $0.contains("phase=loaded") && $0.contains("five.txt") }
            await app.send(["diff 1"])

            // Split by default (`diff_split` is on): the hunk, `a`, three removed lines beside three added ones, `e`.
            let split = await Self.drawn(app, "split")
            app.expect(split.first("diff-drawn")?["rows"] == "6", "three pairs in two columns:\n\(split.text)")
            app.expect(split.first("diff-drawn")?.has("switch") == true, "the header offers Split / Unified:\n\(split.text)")

            // The header's Unified: one column of all eight lines, and the setting says so.
            app.expect(await app.reply("diff-layout unified"), contains: "diff-layout unified")
            let unified = await Self.drawn(app, "unified")
            app.expect(unified.first("diff-drawn")?["rows"] == "9", "every line under the hunk:\n\(unified.text)")
            _ = await app.waitForConfig { $0.range(of: #""diff_split"\s*:\s*false"#, options: .regularExpression) != nil }

            // The setting changed outside the header (Settings, the file) is the same switch.
            await app.send(["set diff_split=true"])
            _ = await Self.drawn(app, "split")

            // Too narrow for two columns (the terminal 340 pt wide): one column though the setting asks for two, and no switch.
            await app.send(["size 1100x900"])
            let narrow = await Self.drawn(app, "unified")
            app.expect(narrow.first("diff-drawn")?.has("switch") == false, "no switch without room:\n\(narrow.text)")
            app.expect(await app.reply("diff-layout split"), contains: "error:")
            app.expect(await app.setting("diff_split") == "bool(true)", "the setting is the person's still")
        }
    }

    // GPUI: `features/git/diff/header.rs:120` (the Binary badge, no line counts, `:143`), `diff/notes.rs:48` and `:78` (one
    // column: "Binary file" with both sizes).
    @Test("a changed binary file: the Binary badge and no line counts, no lines, and its card says Binary file with both sizes")
    @MainActor func binaryDiff() async throws {
        try await Instance.with { app in
            let repo = try app.scratch.repository("bin-repo", files: ["blob.bin": "\u{0}\u{1}\u{2}abc"])
            try app.scratch.write(repo, "blob.bin", "\u{0}\u{1}\u{2}abcdefgh\u{0}")
            await app.terminal(in: repo)
            await app.until("changes") { $0.contains("phase=loaded") && $0.contains("blob.bin") }
            await app.send(["diff 1"])
            let header = Readout(await app.until("diff-header") { Readout($0).all("badge").contains { $0.label == "Binary" } })
            app.expect(header.all("badge").contains { $0.label == "Binary" }, "the badge:\n\(header.text)")
            app.expect(header.first("diff-header")?["counts"] == nil, "a binary file has no lines to count:\n\(header.text)")

            // No lines to lay out: one column whatever the setting says, and the header keeps only its ×.
            let drawn = await Self.drawn(app, "unified")
            app.expect(drawn.first("diff-drawn")?["rows"] == "0", "no rows:\n\(drawn.text)")
            app.expect(drawn.first("diff-drawn")?.has("switch") == false, "no Split / Unified switch:\n\(drawn.text)")
            let note = drawn.all("note").first
            app.expect(note?.label == "Binary file", "the card's words:\n\(drawn.text)")
            app.expect((note?["detail"] ?? "").contains("\u{2192}"), "the old size and the new one:\n\(drawn.text)")
        }
    }
}
