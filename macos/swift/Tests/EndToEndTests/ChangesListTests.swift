import Foundation
import Testing

/// The Changes list as its view builds it (P02): a tree or a flat list, a card per section, a big section folded with its
/// note, the tab's count badge, and a submodule's row with its terminal button. Read with `changes-rows` (the rows
/// `ChangeRows.build` makes, cut into cards as `ChangesListView` cuts them) and `panel-tabs` (the strip's badges).
extension EndToEnd {
    /// The `changes-rows` read-out once the store has `files` files.
    @MainActor static func changeRows(_ app: Instance, files: Int, _ done: (Readout) -> Bool = { _ in true }) async -> Readout {
        await app.until("changes") { $0.contains("phase=loaded") && $0.contains("files=\(files) ") }
        return Readout(await app.until("changes-rows") { done(Readout($0)) })
    }

    // GPUI: `features/git/changes/rows.rs` (a card per section, the tree's single-folder chains joined as `a/b`), core
    // `view::panel::section_folded` and `folded_note` (more than 200 fold, the crowded folder named), the view menu's settings
    // `changes_tree` and `group_untracked` (`changes/view_menu.rs`).
    @Test("Changes as a tree and as a list, untracked files in a card of their own or not, and a section of 201 folded with its note")
    @MainActor func changesTreeAndSections() async throws {
        try await Instance.with { app in
            let repo = try app.scratch.repository(
                "tree-repo", files: ["README.md": "one\n", "src/a/one.rs": "fn one() {}\n", "src/a/two.rs": "fn two() {}\n"])
            for path in ["README.md", "src/a/one.rs", "src/a/two.rs"] { try app.scratch.write(repo, path, "changed\n") }
            for name in ["x.txt", "y.txt"] { try app.scratch.write(repo, "new/\(name)", "new\n") }
            await app.terminal(in: repo)

            // The defaults: a tree, untracked apart.
            let tree = await Self.changeRows(app, files: 5) { $0.first("changes-rows")?.words == ["tree", "grouped"] }
            app.expect(tree.line("band", "changes")?["count"] == "3", "three changes:\n\(tree.text)")
            app.expect(tree.line("band", "untracked")?["count"] == "2", "two untracked in their own card:\n\(tree.text)")
            let folder = tree.all("dir").first { $0.label == "src/a" }
            app.expect(folder?["label"] == "src/a" && folder?["depth"] == "0", "src/ and a/ are one row, src/a:\n\(tree.text)")
            app.expect(tree.all("file").first { $0.label == "src/a/one.rs" }?["depth"] == "1", "a file under its folder:\n\(tree.text)")
            app.expect(tree.all("file").first { $0.label == "README.md" }?["depth"] == "0", "a file at the root:\n\(tree.text)")

            // As a list: no folders, every file at the left with its whole path.
            await app.send(["set changes_tree=false"])
            let list = Readout(await app.until("changes-rows") { Readout($0).first("changes-rows")?.words.first == "list" })
            app.expect(list.all("dir").isEmpty, "a list has no folder rows:\n\(list.text)")
            app.expect(list.all("file").allSatisfy { $0["depth"] == "0" }, "every file at the left:\n\(list.text)")

            // Untracked among the rest: one card.
            await app.send(["set group_untracked=false"])
            let together = Readout(await app.until("changes-rows") { Readout($0).first("changes-rows")?.words.last == "together" })
            app.expect(together.all("band").count == 1 && together.line("band", "all")?["count"] == "5", "one card of five:\n\(together.text)")

            // Over 200 untracked: that card starts folded and says where most of them are.
            await app.send(["set group_untracked=true"])
            for n in 0..<199 { try app.scratch.write(repo, "new/f\(n).txt", "new\n") }
            let big = await Self.changeRows(app, files: 204) { $0.line("band", "untracked")?["count"] == "201" }
            app.expect(big.line("band", "untracked")?.has("folded") == true, "201 untracked start folded:\n\(big.text)")
            app.expect(big.line("band", "changes")?.has("folded") == false, "the three changes stay open:\n\(big.text)")
            let note = Instance.plain(big.all("note").first?.label ?? "")
            app.expect(note.contains("201 of them are in new/") && note.contains("more than 200"), "the folded note:\n\(big.text)")
        }
    }

    // GPUI: the Changes tab's count (`features/git/changes/def.rs:47`: the active terminal's changed files, shown when more
    // than none, whichever tab is on show) and its spoken words (core `view::spoken::panel_tab`).
    @Test("the Changes tab's badge counts the terminal's changed files while another tab is shown, and goes when there are none")
    @MainActor func changesCountBadge() async throws {
        try await Instance.with { app in
            let repo = try app.scratch.repository("badge-repo", files: ["a.txt": "a\n", "b.txt": "b\n"])
            try app.scratch.write(repo, "a.txt", "a\nA\n")
            await app.terminal(in: repo)
            await app.until("changes") { $0.contains("phase=loaded") && $0.contains("files=1 ") }
            let one = Readout(await app.until("panel-tabs") { Readout($0).line("tab", "changes")?["badge"] == "1" })
            app.expect(Instance.plain(one.line("tab", "changes")?["spoken"] ?? "").contains("1"), "VoiceOver hears the count:\n\(one.text)")

            // History is the other tab: it needs no flag (Notes is a feature behind one).
            await app.send(["tab history"])
            await app.until("panel-tabs") { Readout($0).line("tab", "history")?.has("shown") == true }
            try app.scratch.write(repo, "b.txt", "b\nB\n")
            let two = Readout(await app.until("panel-tabs") { Readout($0).line("tab", "changes")?["badge"] == "2" })
            app.expect(two.line("tab", "history")?.has("shown") == true, "counted while History is shown:\n\(two.text)")

            try app.scratch.git(repo, "commit", "-q", "-am", "both")
            let none = Readout(await app.until("panel-tabs") { Readout($0).line("tab", "changes")?["badge"] == "-" })
            app.expect(none.line("tab", "changes")?["badge"] == "-", "no badge with nothing changed:\n\(none.text)")
        }
    }

    // GPUI: `features/git/submodule_row.rs` (the row's words, its button "Open a terminal in …" while checked out), the
    // submodule's diff card (`features/git/diff/submodule.rs`: the commits it moved by).
    @Test("a submodule moved by a commit: its row says so and offers a terminal, its diff counts the commit, the button opens one there")
    @MainActor func changesSubmodule() async throws {
        try await Instance.with { app in
            let lib = try app.scratch.repository("sub-lib", files: ["lib.c": "int x;\n"])
            let repo = try app.scratch.repository("super-repo", files: ["README.md": "one\n"])
            // A local clone only (file protocol), never a network.
            try app.scratch.git(repo, "-c", "protocol.file.allow=always", "submodule", "add", "-q", lib.path, "vendor/lib")
            try app.scratch.git(repo, "commit", "-q", "-m", "add the submodule")
            let inside = repo.appendingPathComponent("vendor/lib")
            try app.scratch.write(inside, "lib.c", "int y;\n")
            try app.scratch.git(inside, "commit", "-q", "-am", "inside change")
            let before = (await app.terminal(in: repo)).live.count

            let rows = await Self.changeRows(app, files: 1) { $0.all("file").contains { $0.label == "vendor/lib" } }
            let row = rows.all("file").first { $0.label == "vendor/lib" }
            app.expect(row?["submodule"] != nil, "the row says what the parent sees of it:\n\(rows.text)")
            app.expect(row?.has("enter") == true, "checked out: the row offers a terminal in it:\n\(rows.text)")

            await app.send(["diff 1"])
            let header = Readout(await app.until("diff-header") { Readout($0).first("submodule") != nil })
            app.expect(header.first("submodule")?["ahead"] == "1" && header.first("submodule")?["behind"] == "0", "one commit ahead:\n\(header.text)")

            app.expect(await app.reply("submodule-enter vendor/lib"), contains: "submodule-enter vendor/lib")
            let opened = await app.until("rows") { Listing($0).live.count == before + 1 }
            let made = Listing(opened).rows.first { $0.folder.hasSuffix("/vendor/lib") }
            app.expect(made != nil, "a terminal in the submodule's folder:\n\(opened)")
            app.expect(await app.reply("submodule-enter README.md"), contains: "error:")
        }
    }
}
