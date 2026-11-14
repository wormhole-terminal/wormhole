import Foundation
import Testing

/// The Files tab, read through `files` (the tab as text: its root and how it was chosen, the pins, the viewer, the rows) in a
/// throwaway repository. A rename writes to the disk of the scratch folder only.
extension EndToEnd {
    /// The folder `files` says the tree is of: its first line, `files: <root> (following|chosen) pinned=…`.
    static func filesRoot(_ listing: String) -> String {
        let head = listing.split(separator: "\n").first.map(String.init) ?? ""
        return head.replacingOccurrences(of: "files: ", with: "").components(separatedBy: " (").first ?? ""
    }

    @Test("Files lists the repository from a subfolder, renames a file on disk, refuses a taken name, and its viewer covers the terminal")
    @MainActor func filesRename() async throws {
        try await Instance.with { app in
            let repo = try app.scratch.repository(
                "files-repo", files: ["README.md": "one\n", "src/a.txt": "a\n", "src/c.txt": "c\n", "docs/b.md": "b\n"])
            await app.flags(["files": true])
            await app.terminal(in: repo.appendingPathComponent("src"))
            let tree = await app.until("files") { $0.contains("src") && $0.contains("docs") }
            let head = tree.split(separator: "\n").first.map(String.init) ?? ""
            app.expect(head.hasSuffix("/files-repo (following) pinned=-"), "the repository's root, following the terminal:\n\(tree)")

            await app.send(["files-expand src"])
            await app.until("files") { $0.contains("a.txt") && $0.contains("c.txt") }
            await app.send(["files-rename src/a.txt renamed.txt"])
            app.expect(await app.waitFor(repo.appendingPathComponent("src/renamed.txt")), "renamed on disk")
            app.expect(!FileManager.default.fileExists(atPath: repo.appendingPathComponent("src/a.txt").path), "the old name is gone")
            await app.until("files") { $0.contains("renamed.txt") && !$0.contains("a.txt") }

            // A name that is taken: nothing changes on disk, and the chip says why.
            await app.send(["files-rename src/renamed.txt c.txt"])
            await app.until("notice chip") { $0.contains("something with that name exists already") }
            app.expect(FileManager.default.fileExists(atPath: repo.appendingPathComponent("src/renamed.txt").path), "the file kept its name")
            app.expect((try? String(contentsOf: repo.appendingPathComponent("src/c.txt"), encoding: .utf8)) == "c\n", "c.txt is untouched")

            // The viewer goes over the terminal, stays there on another tab, and Esc gives the terminal its keys back.
            await app.send(["files-open README.md"])
            app.expect(await app.until("files") { $0.contains("viewer: README.md") }, contains: "viewer: README.md")
            await app.until("keys") { $0.contains("+cover") }
            await app.send(["tab notes"])
            app.expect(await app.reply("keys"), contains: "+cover")
            await app.send(["key esc"])
            let keys = await app.until("keys") { !$0.contains("+cover") }
            app.expect(keys.hasPrefix("focus=terminal"), "the terminal has the keys:\n\(keys)")
        }
    }

    @Test("Files: a chosen folder, following the terminal again, a pin kept in config.json and following a rename, the path menus")
    @MainActor func filesPinsAndMenus() async throws {
        try await Instance.with { app in
            let repo = try app.scratch.repository("pins-repo", files: ["README.md": "one\n", "src/a.txt": "a\n", "docs/b.md": "b\n"])
            await app.flags(["files": true])
            await app.terminal(in: repo)
            let first = await app.until("files") { $0.contains("pins-repo (following)") }
            // The root as the tab has it (the terminal's folder as the system names it: `/private/tmp/…` for `/tmp/…`).
            let root = Self.filesRoot(first)
            await app.send(["files-root \(repo.appendingPathComponent("docs").path)"])
            await app.until("files") { ($0.split(separator: "\n").first ?? "").contains("/docs (chosen)") }
            // Another terminal has its own tree; the first keeps its chosen folder.
            guard let a = (await app.rows()).selected else { return }
            await app.terminal(in: repo)
            await app.until("files") { $0.contains("pins-repo (following)") }
            await app.send(["select \(a)"])
            await app.until("files") { ($0.split(separator: "\n").first ?? "").contains("/docs (chosen)") }
            await app.send(["files-follow"])
            await app.until("files") { $0.contains("pins-repo (following)") }

            await app.send(["files-pin \(root)/src"])
            await app.until("files") { $0.contains("pinned=src") }
            _ = await app.waitForConfig { $0.contains("files_pinned") && $0.contains("src") }
            // Renaming a pinned folder moves its pin to the new name.
            await app.send(["files-rename src source"])
            app.expect(await app.waitFor(repo.appendingPathComponent("source")), "renamed on disk")
            await app.until("files") { $0.contains("pinned=source") }
            await app.send(["files-pin \(root)/source"])
            await app.until("files") { $0.contains("pinned=-") }

            let menu = await app.reply("files-menu")
            app.expect(menu, contains: "Follow")
            await app.send(["menu-close"])
            let path = await app.reply("path-menu")
            app.expect(path, contains: "Copy Path")
            app.expect(await app.reply("menu-pick Copy Path"), contains: "picked Copy Path")
            app.expect(await app.reply("pasteboard"), contains: "pins-repo")
            await app.until("notice chip") { $0.contains("Copied path") }
        }
    }

    @Test(
        "a file whose name is not UTF-8 is listed beside its neighbours with the replacement character",
        .disabled("waits for a fixture: a volume that takes a file name that is not UTF-8 (APFS, where the scratch folder is, refuses to make one)"))
    @MainActor func filesRawName() async throws {
        try await Instance.with { app in
            let folder = app.scratch.home.appendingPathComponent("raw-names")
            try app.scratch.write(folder, "a.txt", "a\n")
            try app.scratch.rawFile(in: folder, named: Array("a".utf8) + [0xFF] + Array("b.txt".utf8))
            await app.flags(["files": true])
            await app.terminal(in: folder)
            let listed = await app.until("files") { $0.contains("a.txt") && $0.contains("b.txt") }
            app.expect(listed, contains: "a\u{FFFD}b.txt")
        }
    }
}
