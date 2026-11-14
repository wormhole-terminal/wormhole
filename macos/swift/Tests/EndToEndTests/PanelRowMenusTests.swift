import Foundation
import Testing

/// The right-click menus of the panel's rows, found where a right-click finds them (`changes-menu`, `history-menu`,
/// `files-row-menu`: the row's own frame, the menu kept unshown for `menu-pick`). A reply is `menu:` then a line an item,
/// `n. title [key] [x] [off]`, a submenu's items indented. TO-VERIFY.md lines 52, 235, 238, 333, 335.
extension EndToEnd {
    /// The menu's item lines once the row is on screen and answers.
    @MainActor static func panelRowMenu(_ app: Instance, _ command: String) async -> String {
        await app.until(command) { $0.hasPrefix("menu:") }
    }

    @Test("a changed file's menu: Reveal, Copy Path (to the instance's clipboard), Copy Relative Path, File History that opens History (52, 235, 238)")
    @MainActor func changesRowMenu() async throws {
        try await Instance.with { app in
            let repo = try app.scratch.repository("menu-repo", files: ["README.md": "one\n"])
            try app.scratch.write(repo, "README.md", "one\ntwo\n")
            await app.terminal(in: repo)
            await app.send(["tab changes"])
            await app.until("changes") { $0.contains("phase=loaded") && $0.contains("files=1") }
            let menu = await Self.panelRowMenu(app, "changes-menu 1")
            for item in ["Reveal in Finder", "Copy Path", "Copy Relative Path", "File History"] { app.expect(menu, contains: item) }

            app.expect(await app.reply("menu-pick Copy Path"), contains: "picked Copy Path")
            app.expect(await app.reply("pasteboard"), contains: "menu-repo/README.md")
            _ = await Self.panelRowMenu(app, "changes-menu README.md")
            await app.send(["menu-pick File History"])
            await app.until("state") { $0.contains("tab=history") }

            // With History switched off the item is not offered.
            await app.send(["set features={\"history\":false}", "tab changes"])
            await app.until("changes-menu 1") { $0.hasPrefix("menu:") && !$0.contains("File History") }
            await app.send(["menu-close"])
        }
    }

    @Test("a commit's menu has the five copies; the open commit's block adds Show more and its parent (333)")
    @MainActor func historyRowMenus() async throws {
        try await Instance.with { app in
            let repo = try app.scratch.repository("hist-menu", files: ["a.txt": "a\n"], message: "first commit")
            let body = String(repeating: "A long line of the body that goes on and on. ", count: 12)
            try app.scratch.write(repo, "b.txt", "b\n")
            try app.scratch.git(repo, "add", "-A")
            try app.scratch.git(repo, "commit", "-q", "-m", "second commit", "-m", body)
            await app.terminal(in: repo)
            await app.send(["tab history"])
            await app.until("history-state") { $0.contains("commits=2") }

            let row = await Self.panelRowMenu(app, "history-menu 1")
            for item in ["Copy Hash", "Copy Short Hash", "Copy Message", "Copy Subject", "Copy Author"] { app.expect(row, contains: item) }
            app.expect(!row.contains("Open Parent"), "the row's menu is the copies alone:\n\(row)")
            await app.send(["menu-close", "history 1"])
            await app.until("commit-details") { $0.contains("subject:") }
            let block = await Self.panelRowMenu(app, "history-menu details")
            app.expect(block, contains: "Show more")
            app.expect(block, contains: "Copy Hash")
            app.expect(block, contains: "Open Parent")
            await app.send(["menu-close"])
        }
    }

    @Test("a Files row's menu: Open in, Reveal, Copy Path, Rename…; a folder's adds Pin Folder (335)")
    @MainActor func filesRowMenu() async throws {
        try await Instance.with { app in
            let repo = try app.scratch.repository("files-menu", files: ["README.md": "one\n", "src/a.txt": "a\n"])
            await app.terminal(in: repo)
            await app.send(["tab files"])
            await app.until("files") { $0.contains("README.md") && $0.contains("src") }
            let file = await Self.panelRowMenu(app, "files-row-menu README.md")
            for item in ["Open in", "Reveal in Finder", "Copy Path", "Rename"] { app.expect(file, contains: item) }
            app.expect(!file.contains("Pin Folder"), "a file has no pin:\n\(file)")
            await app.send(["menu-close"])
            let folder = await Self.panelRowMenu(app, "files-row-menu src")
            app.expect(folder, contains: "Pin Folder")
            app.expect(folder, contains: "Open in")
            await app.send(["menu-close"])
        }
    }
}
