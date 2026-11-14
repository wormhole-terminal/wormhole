import Foundation
import Testing

/// The open diff read with `diff-header`, `diff-step` and `diff-spans` (`Readout`'s shape): the header's position, badges and
/// commit, the arrows wrapping round, and each line's colour runs by token as the core painted them. TO-VERIFY.md lines 240,
/// 241, 468, 578, 579, 580, 581, 588.
extension EndToEnd {
    /// The diff of the file at `path` (the n-th of `changes`), once it is shown and its rows are cut.
    @MainActor static func openDiff(_ app: Instance, _ files: [String], _ path: String) async -> Readout {
        let n = (files.firstIndex { $0.contains(" \(path) ") } ?? -1) + 1
        await app.send(["diff \(n)"])
        await app.until("changes") { $0.contains("open=\(path) diff=shown") }
        return Readout(await app.until("diff-spans 0 400") { Readout($0).first("diff-spans")?["rows"] != nil })
    }

    /// The token names a read-out's lines carry, by line kind.
    static func tokens(_ spans: Readout, kind: String) -> Set<String> {
        Set(spans.all("line").filter { $0.words.dropFirst().first == kind }.flatMap { ($0["spans"] ?? "").split(separator: ",").compactMap { $0.split(separator: ":").last.map(String.init) } })
    }

    @Test("a Rust, a TypeScript and a plain text diff: code lines carry the theme's tokens on both sides, plain text none (578, 579, 580, 588)")
    @MainActor func diffSyntaxSpans() async throws {
        try await Instance.with { app in
            let repo = try app.scratch.repository(
                "spans-repo",
                files: ["lib.rs": "fn one() -> u32 {\n    1\n}\n", "app.ts": "const one: number = 1;\n", "notes.txt": "plain words\n"])
            try app.scratch.write(repo, "lib.rs", "fn two() -> u32 {\n    \"two\".len() as u32\n}\n")
            try app.scratch.write(repo, "app.ts", "const two: string = \"two\";\n")
            try app.scratch.write(repo, "notes.txt", "other words\n")
            await app.terminal(in: repo)
            let listed = await app.until("changes") { $0.contains("phase=loaded") && $0.contains("files=3") }
            let files = Self.changedFiles(listed)

            let rust = await Self.openDiff(app, files, "lib.rs")
            app.expect(rust.first("diff-spans")?.has("colored-by-core") == true, "the core painted it:\n\(rust.text)")
            app.expect(Self.tokens(rust, kind: "added").contains("keyword"), "an added Rust line has a keyword:\n\(rust.text)")
            app.expect(!Self.tokens(rust, kind: "removed").isEmpty, "a removed line is coloured from the old file:\n\(rust.text)")
            let header = Readout(await app.reply("diff-header"))
            app.expect(header.first("diff-header")?["position"] != nil, "File N of M:\n\(header.text)")

            let script = await Self.openDiff(app, files, "app.ts")
            app.expect(!Self.tokens(script, kind: "added").isEmpty, "TypeScript is coloured:\n\(script.text)")

            let plain = await Self.openDiff(app, files, "notes.txt")
            app.expect(plain.all("line").allSatisfy { ($0["spans"] ?? "").isEmpty }, "a text file is plain:\n\(plain.text)")
        }
    }

    @Test("the header's arrows wrap round the list; a file moved to another folder says Renamed from (240, 241)")
    @MainActor func diffHeaderStepAndRename() async throws {
        try await Instance.with { app in
            let repo = try app.scratch.repository("step-repo", files: ["a/same.txt": "same\n", "b.txt": "b\n"])
            // git does not make the folder a file moves into.
            try FileManager.default.createDirectory(at: repo.appendingPathComponent("c"), withIntermediateDirectories: true)
            try app.scratch.git(repo, "mv", "a/same.txt", "c/same.txt")
            try app.scratch.write(repo, "b.txt", "b\nb\n")
            await app.terminal(in: repo)
            let listed = await app.until("changes") { $0.contains("phase=loaded") && $0.contains("same.txt") && $0.contains("b.txt") }
            let files = Self.changedFiles(listed)
            let count = files.count
            _ = await Self.openDiff(app, files, "b.txt")
            // To the list's first file by ›, which wraps from the last, then ‹ from the first.
            for _ in 0..<count {
                let position = Readout(await app.reply("diff-header")).first("diff-header")?["position"]
                if position == "1 of \(count)" { break }
                await app.send(["diff-step next"])
            }
            let first = Readout(await app.until("diff-header") { Readout($0).first("diff-header")?["position"] == "1 of \(count)" })
            app.expect(first.first("diff-header") != nil, "the first file:\n\(first.text)")
            app.expect(await app.reply("diff-step prev"), contains: "diff-step prev")
            let last = Readout(await app.until("diff-header") { Readout($0).first("diff-header")?["position"] == "\(count) of \(count)" })
            app.expect(last.first("diff-header") != nil, "‹ on the first goes round to the last")
            await app.send(["diff-step next"])
            await app.until("diff-header") { Readout($0).first("diff-header")?["position"] == "1 of \(count)" }

            let moved = files.firstIndex { $0.contains("c/same.txt") }.map { $0 + 1 }
            if let moved {
                await app.send(["diff \(moved)"])
                let header = Readout(await app.until("diff-header") { Readout($0).first("diff-header")?.label == "c/same.txt" })
                app.expect(header.all("badge").contains { $0.label == "Renamed from same.txt" }, "the old name, though it kept its name:\n\(header.text)")
            } else {
                app.fail("the moved file is listed:\n\(listed)")
            }
        }
    }

    @Test("a diff over 20,000 lines is not highlighted, and says so (581)")
    @MainActor func diffTooLongForColours() async throws {
        try await Instance.with { app in
            let repo = try app.scratch.repository("long-repo", files: ["long.rs": "fn x() {}\n"])
            try app.scratch.write(repo, "long.rs", (0..<20_100).map { "fn f\($0)() {}" }.joined(separator: "\n") + "\n")
            await app.terminal(in: repo)
            await app.until("changes") { $0.contains("phase=loaded") && $0.contains("long.rs") }
            await app.send(["diff 1"])
            let header = Readout(await app.until("diff-header", timeout: 40) { Readout($0).all("badge").contains { $0.label == "No highlighting" } })
            app.expect(header.all("badge").contains { $0.label == "No highlighting" }, "the badge:\n\(header.text)")
            app.expect(Readout(await app.reply("diff-spans 0 50")).all("line").allSatisfy { ($0["spans"] ?? "").isEmpty }, "the lines are plain")
        }
    }

    @Test("from History a file's diff names the commit after File N of M (468)")
    @MainActor func diffHeaderFromHistory() async throws {
        try await Instance.with { app in
            let repo = try app.scratch.repository("hist-diff", files: ["a.txt": "a\n"])
            try app.scratch.write(repo, "a.txt", "a\nb\n")
            try app.scratch.git(repo, "commit", "-q", "-am", "second")
            await app.terminal(in: repo)
            await app.send(["tab history"])
            let state = await app.until("history-state") { $0.contains("commits=2") }
            // The commit's files are listed a moment after it opens; one is clicked once it is there.
            await app.send(["history 1"])
            await app.until("commit-details") { $0.contains("subject:") }
            await app.send(["history-file 1"])
            await app.until("history-state") { $0.contains("diff=shown") }
            let header = Readout(await app.until("diff-header") { Readout($0).first("diff-header")?["commit"] != nil })
            let short = header.first("diff-header")?["commit"] ?? ""
            app.expect(short.count == 7, "the commit's short hash: \(short)\n\(state)")
            app.expect(header.first("diff-header")?.words.first == "history", "History's diff:\n\(header.text)")
        }
    }
}
