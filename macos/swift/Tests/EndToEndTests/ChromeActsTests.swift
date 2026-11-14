import Foundation
import Testing

/// The window's small parts read and pressed through their own closures: the find bar (`find-state`), the branch chip
/// (`branch-chip`), a group's popover (`group-popover`, then `popover`), the list's no-agents note (`sidebar-tip`), the +
/// menu (`plus-menu`), a diff line's comment (`comment-at`, `comment`, `review`), a copy's chip (`notice chip`), the
/// status bar of another window's terminal, a group's name field, a selection stashed as a note. TO-VERIFY.md lines 27, 53,
/// 138, 168, 169, 179, 181, 182, 195, 431, 769, 771, 827, 890, 960.
extension EndToEnd {
    @Test("⌘F with a word selected opens Find with that word as its query (27)")
    @MainActor func findSeededBySelection() async throws {
        try await Instance.with { app in
            await app.send(["text seedword", "key return"])
            let screen = await app.screen(showing: "seedword")
            let lines = screen.split(separator: "\n", omittingEmptySubsequences: false).map(String.init)
            guard let row = lines.firstIndex(where: { $0.contains("seedword") }), let found = lines[row].range(of: "seedword") else {
                return app.fail("the word on screen:\n\(screen)")
            }
            let column = lines[row].distance(from: lines[row].startIndex, to: found.lowerBound)
            await app.send(["select-cells \(row) \(column) \(row) \(column + 7)", "key cmd-f"])
            let find = Readout(await app.until("find-state") { Readout($0).first("find-state")?.has("open") == true })
            app.expect(find.first("find-state")?["query"] == "seedword", "the selection is the query:\n\(find.text)")
        }
    }

    @Test("the branch chip opens Changes and its compare list; in a folder that is no repository the request ends (769, 960)")
    @MainActor func branchChip() async throws {
        try await Instance.with { app in
            let repo = try app.scratch.repository("chip-repo")
            let listing = await app.terminal(in: repo)
            guard let id = listing.selected else { return }
            await app.send(["tab notes", "branch-chip"])
            await app.until("state") { $0.contains("tab=changes") }
            app.expect(await app.reply("branch-chip state"), contains: "picker=\(id)")

            let plain = app.scratch.home.appendingPathComponent("not-a-repo")
            try FileManager.default.createDirectory(at: plain, withIntermediateDirectories: true)
            await app.terminal(in: plain)
            await app.until("changes") { $0.contains("notRepository") }
            await app.send(["branch-chip"])
            await app.until("branch-chip state") { $0.contains("picker=-") }
        }
    }

    @Test("a group's popover lists its colours with No colour first and its actions; closing it gives the terminal the keys (169)")
    @MainActor func groupPopover() async throws {
        try await Instance.with { app in
            let rows = await app.rows()
            guard let first = rows.live.first else { return }
            await app.send(["group \(first)"])
            guard let group = Listing(await app.until("rows") { !Listing($0).groups.isEmpty }).groups.first else { return }
            await app.send(["group-popover \(group.id)"])
            let popover = await app.until("popover") { $0.contains("colours:") }
            app.expect(popover.split(separator: "\n").first { $0.hasPrefix("colours:") }?.dropFirst("colours: ".count).hasPrefix("No colour") == true, "No colour first:\n\(popover)")
            for action in ["- New terminal in group", "- Move group to new window", "- Pin group"] { app.expect(popover, contains: action) }
            await app.send(["popover-close"])
            await app.until("keys") { $0.hasPrefix("focus=terminal") }
        }
    }

    @Test("a first run that ends on the primer puts up the no-agents note; × offers Don't show again, which Reset tips undoes (138, 181, 827)")
    @MainActor func noAgentsTip() async throws {
        try await Instance.with { app in
            _ = await Self.page(app, "advanced") { $0.line("row", "reset-tips") != nil }
            app.expect(Readout(await app.reply("settings-page")).line("row", "reset-tips")?.has("off") == true, "Reset tips is off while no tip was closed")
            _ = await Self.onboarding(app, at: "primer")
            await app.send(["onboarding-press main"])
            await app.until("sidebar-tip") { $0 == "sidebar-tip note" }
            await app.send(["sidebar-tip close"])
            await app.until("sidebar-tip") { $0 == "sidebar-tip offer" }
            await app.send(["sidebar-tip never"])
            await app.until("sidebar-tip") { $0 == "sidebar-tip off" }
            let page = await Self.page(app, "advanced") { $0.line("row", "reset-tips")?.has("off") == false }
            app.expect(page.line("row", "reset-tips") != nil, "Reset tips is on after a tip was closed for good")
            _ = await Self.press(app, "reset-tips")
            await app.until("settings-page") { Readout($0).line("row", "reset-tips")?.has("off") == true }
            await app.send(["sidebar-tip settings"])
            await app.until("settings-page") { Readout($0).first("settings-page")?.words.first == "agents" }
        }
    }

    @Test("the + menu lists the shells with the default checked; a choice opens a terminal (179, 182)")
    @MainActor func plusMenu() async throws {
        try await Instance.with { app in
            let menu = Readout(await app.reply("plus-menu"))
            let shells = menu.lines.dropFirst()
            app.expect(shells.filter { $0.has("default") }.count == 1, "one default:\n\(menu.text)")
            guard let choice = shells.first(where: { $0.kind != "-" })?.kind else { return app.fail("a shell with a key:\n\(menu.text)") }
            let before = (await app.rows()).live.count
            app.expect(await app.reply("plus-menu pick \(choice)"), contains: "plus-menu picked")
            await app.until("rows") { Listing($0).live.count == before + 1 }
        }
    }

    @Test("a comment on a changed line: saved once, the dialog gone; a long line's quote is cut at 160 scalars (771, 890)")
    @MainActor func commentOnLine() async throws {
        try await Instance.with { app in
            let long = String(repeating: "e\u{301}", count: 120) + " 🙂🙂🙂   "
            let repo = try app.scratch.repository("review-repo", files: ["a.txt": "one\n"])
            try app.scratch.write(repo, "a.txt", "one\n\(long)\n")
            await app.terminal(in: repo)
            await app.until("changes") { $0.contains("phase=loaded") && $0.contains("a.txt") }
            await app.send(["diff 1"])
            let spans = Readout(await app.until("diff-spans 0 40") { Readout($0).all("line").contains { $0.words.dropFirst().first == "added" } })
            guard let row = spans.all("line").first(where: { $0.words.dropFirst().first == "added" })?.words.first else { return }
            app.expect(await app.reply("comment-at \(row)"), contains: "comment-at a.txt:")
            app.expect(await app.reply("comment hello"), contains: "1 comment")
            app.expect(await app.reply("comment again"), contains: "error: no comment dialog is open")
            let message = await app.reply("review")
            let quote = message.split(separator: "\n").first { $0.trimmingCharacters(in: .whitespaces).hasPrefix(">") }.map(String.init) ?? ""
            let quoted = quote.trimmingCharacters(in: .whitespaces).dropFirst(2)
            app.expect(quoted.unicodeScalars.count <= 160 && !quoted.hasSuffix(" "), "cut at 160 scalars, trimmed: \(quoted.unicodeScalars.count)\n\(message)")
        }
    }

    @Test("a commit's Copy Hash puts the hash on the instance's clipboard and the chip says what was copied (53)")
    @MainActor func historyCopyChip() async throws {
        try await Instance.with { app in
            let repo = try app.scratch.repository("copy-repo")
            await app.terminal(in: repo)
            await app.send(["tab history"])
            await app.until("history-state") { $0.contains("commits=1") }
            let hash = try app.scratch.git(repo, "rev-parse", "HEAD").trimmingCharacters(in: .whitespacesAndNewlines)
            _ = await Self.panelRowMenu(app, "history-menu 1")
            await app.send(["menu-pick Copy Hash"])
            await app.until("pasteboard") { $0.contains(hash) }
            app.expect(await app.until("notice chip") { $0.contains("Copied") }, contains: "Copied")
        }
    }

    @Test("window 1's status bar shows window 2's terminal that needs input (195)")
    @MainActor func statusAcrossWindows() async throws {
        try await Instance.with(shell: .agent("codex")) { app in
            await app.send(["status-bar on", "new-window"])
            await app.until("sessions") { $0.split(separator: "\n").count == 2 }
            guard let theirs = (await app.rows(window: 2)).live.first else { return }
            try await app.report(agent: "codex", event: "PermissionRequest", terminal: theirs, becomes: "needsInput")
            let bar = Readout(await app.until("status-items", window: 1) { Readout($0).first("attention") != nil })
            app.expect(bar.first("attention")?.words.first == String(theirs), "the other window's terminal:\n\(bar.text)")
        }
    }

    @Test("a group's name field: Esc keeps the old name, Return the new one, a blank name the old one (168)")
    @MainActor func groupRenameField() async throws {
        try await Instance.with { app in
            guard let first = (await app.rows()).live.first else { return }
            await app.send(["group \(first)"])
            guard let group = Listing(await app.until("rows") { !Listing($0).groups.isEmpty }).groups.first else { return }
            func rename(_ keys: [String]) async -> String {
                await app.send(["group-menu \(group.id)", "menu-pick Rename…", "wait-idle 3000"] + keys)
                return Listing(await app.reply("rows")).groups.first { $0.id == group.id }?.name ?? ""
            }
            app.expect(await rename(["text thrown", "key esc"]) == group.name, "Esc keeps \(group.name)")
            app.expect(await rename(["text Kept", "key return"]) == "Kept", "Return keeps the new name")
            app.expect(await rename(["key backspace", "key return"]) == "Kept", "a blank name keeps the old one")
            app.expect((await app.reply("keys")).hasPrefix("focus=terminal"), "the keys are back on the terminal")
        }
    }

    @Test("⌥⌘S with text selected in the terminal makes it a note and shows Notes (431)")
    @MainActor func stashSelection() async throws {
        try await Instance.with { app in
            await app.send(["text stash-me", "key return"])
            let screen = await app.screen(showing: "stash-me")
            let lines = screen.split(separator: "\n", omittingEmptySubsequences: false).map(String.init)
            guard let row = lines.firstIndex(where: { $0.hasPrefix("stash-me") }) else { return app.fail("on screen:\n\(screen)") }
            await app.send(["select-cells \(row) 0 \(row) 7", "key alt-cmd-s"])
            let notes = await app.until("notes") { $0.contains("stash-me") }
            app.expect(notes, contains: "stash-me")
            await app.until("state") { $0.contains("tab=notes") }
        }
    }
}
