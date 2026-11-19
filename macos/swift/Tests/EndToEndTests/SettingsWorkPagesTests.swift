import Foundation
import Testing

/// Settings' Work pages (Terminal, Terminals & sidebar, Changes & diffs) changed the way a person changes them: a click on the
/// control itself (`settings-press`, at the frame the row records; along a segmented control or at a stepper's arrow with
/// `at <x> [y]`), then the value read where it lands (`config.json`) and where it acts: the tab bar, the open diff's header,
/// the panel's and the sidebar's own models (`look-keys` reads the stores those views draw from), a program's copy.
extension EndToEnd {
    @Test("Terminals & sidebar, clicked: Tabs puts the terminals in a bar; the change count's rows leave with its switch and its scope; the dot, the ⌘ hints and the numbers follow")
    @MainActor func sidebarPageClicked() async throws {
        try await Instance.with { app in
            await app.send(["key cmd-t"])
            let two = Listing(await app.until("rows") { Listing($0).live.count == 2 })

            // Terminals in → Tabs (core `settings_index/table/sessions.rs:17`; GPUI's window then draws the bar, `window/menu_state.rs:84`).
            await app.reveal("terminals-in")
            await app.pressAlong("terminals-in", at: Self.rightSegment) { await app.setting("terminals_in") == #"string("tabs")"# }
            _ = await app.waitForConfig { $0.range(of: #""terminals_in"\s*:\s*"tabs""#, options: .regularExpression) != nil }
            let bar = Readout(await app.until("tab-bar") { Readout($0).first("tabs")?.has("on-top") == true && Readout($0).all("tab").count == 2 })
            app.expect(bar.first("tab-bar")?.has("shown") == true, "the two terminals are tabs in a bar on show:\n\(bar.text)")

            // The change count off: its own rows are not on the page while it is (core `settings_index/table/mod.rs:64`, `sessions.rs:59-83`).
            await app.reveal("sidebar-changes")
            _ = await Self.press(app, "sidebar-changes")
            await app.until("get sidebar_changes") { $0.hasSuffix("bool(false)") }
            let off = Readout(await app.until("settings-page") { Readout($0).line("row", "sidebar-changes")?["value"] == "off" })
            for row in ["sidebar-changes-scope", "sidebar-changes-plain", "sidebar-changes-shared", "sidebar-changes-remember"] {
                app.expect(off.line("row", row) == nil, "\(row) is not on the page while the count is off:\n\(off.text)")
            }
            _ = await app.waitForConfig { $0.range(of: #""sidebar_changes"\s*:\s*false"#, options: .regularExpression) != nil }
            _ = await Self.press(app, "sidebar-changes")
            await app.until("settings-page") { Readout($0).line("row", "sidebar-changes-plain") != nil }

            // Scope → Repository: what counts only for one terminal leaves the page (`sessions.rs:67, 75`); the sidebar counts by the repository.
            await app.reveal("sidebar-changes-scope")
            await app.pressAlong("sidebar-changes-scope", at: Self.rightSegment) { await app.setting("sidebar_changes_scope") == #"string("repository")"# }
            let scoped = Readout(await app.until("settings-page") { Readout($0).line("row", "sidebar-changes-plain") == nil })
            app.expect(scoped.line("row", "sidebar-changes-shared") == nil, "the shared rule went with it:\n\(scoped.text)")
            app.expect(scoped.line("row", "sidebar-changes-remember") != nil, "Remember stays while the count is on:\n\(scoped.text)")
            await app.until("look-keys") { $0.contains("scope=repository") }

            // The status dot and the ⌘ hints: switches the sidebar's own appearance follows.
            for (row, key, word) in [("sidebar-dot", "sidebar_status_dot", "status_dot"), ("cmd-hints", "sidebar_cmd_hints", "cmd_hints")] {
                await app.reveal(row)
                _ = await Self.press(app, row)
                await app.until("look-keys") { $0.contains("\(word)=false") }
                _ = await app.waitForConfig { $0.range(of: "\"\(key)\"\\s*:\\s*false", options: .regularExpression) != nil }
            }

            // ⌘1–9 follow the list: the core's workspace counts that way, and a pinned terminal, now first, is ⌘1
            // (core `workspace/panel.rs:57`, `workspace/tests/group_actions.rs:176`).
            await app.reveal("sidebar-numbers")
            await app.pressAlong("sidebar-numbers", at: Self.rightSegment) { await app.setting("sidebar_numbers") == #"string("list")"# }
            await app.until("look-keys") { $0.contains("numbers_follow_list=true") }
            guard let second = two.live.last else { return }
            await app.send(["pin \(second)"])
            await app.until("rows") { Listing($0).row(second)?.number == 1 }
        }
    }

    @Test("Changes & diffs, clicked: Unified and Wrap reach the open diff's header, the stepper's wider tab widens its lines, List, the untracked group and MiB reach the panel")
    @MainActor func gitPageClicked() async throws {
        try await Instance.with { app in
            let repo = try app.scratch.repository("git-page", files: ["t.txt": "a\n"])
            try app.scratch.write(repo, "t.txt", "a\n\t\t\tx\n")
            await app.terminal(in: repo)
            await app.until("changes") { $0.contains("phase=loaded") && $0.contains("t.txt") }
            await app.send(["diff 1"])
            let first = Readout(await app.until("diff-header") { Readout($0).first("diff-header")?["widest"] != nil })
            let header = first.first("diff-header")
            app.expect(header?.has("split") == true && header?.has("wrap") == false, "split and not wrapped by default:\n\(first.text)")
            let widest = Int(header?["widest"] ?? "") ?? 0

            // Unified (GPUI `settings_window/sections/git.rs:31`; the header's own layout buttons, `features/git/diff/header.rs:197`).
            await app.reveal("diff-layout")
            await app.pressAlong("diff-layout", at: Self.rightSegment) { await app.setting("diff_split") == "bool(false)" }
            await app.until("diff-header") { Readout($0).first("diff-header").map { $0["widest"] != nil && !$0.has("split") } == true }
            _ = await app.waitForConfig { $0.range(of: #""diff_split"\s*:\s*false"#, options: .regularExpression) != nil }

            // Wrap long lines (`git.rs:34`, `header.rs:95`).
            await app.reveal("diff-wrap")
            _ = await Self.press(app, "diff-wrap")
            await app.until("diff-header") { Readout($0).first("diff-header")?.has("wrap") == true }

            // Tab width: the stepper's up arrow (`git.rs:38`); the three tabs take more columns.
            let line = await app.reveal("tab-width")
            let frame = line.components(separatedBy: " | ").last?.split(separator: " ").compactMap { Double($0) } ?? []
            let arrow = frame.count == 4 && frame[2] > 0 ? max(0.5, 1 - 5 / frame[2]) : 0.95
            await app.pressAlong("tab-width", at: [(arrow, 0.25), (arrow, 0.3), (arrow, 0.2), (arrow, 0.35)]) {
                (Self.number(await app.setting("tab_width")) ?? 0) > 4
            }
            await app.until("diff-header") { (Int(Readout($0).first("diff-header")?["widest"] ?? "") ?? 0) > widest }
            _ = await app.waitForConfig { $0.range(of: #""tab_width"\s*:\s*[5-9]"#, options: .regularExpression) != nil }

            // List, not the tree; the untracked files not grouped; sizes in MiB: what the Changes and Files views draw from.
            await app.reveal("changes-view")
            await app.pressAlong("changes-view", at: Self.leftSegment) { await app.setting("changes_tree") == "bool(false)" }
            await app.until("look-keys") { $0.contains("changes_tree=false") }
            await app.reveal("group-untracked")
            _ = await Self.press(app, "group-untracked")
            await app.until("look-keys") { $0.contains("group_untracked=false") }
            await app.reveal("size-units")
            await app.pressAlong("size-units", at: Self.rightSegment) { await app.setting("size_units") == #"string("binary")"# }
            await app.until("look-keys") { $0.contains("binary_sizes=true") }
            _ = await app.waitForConfig { $0.range(of: #""size_units"\s*:\s*"binary""#, options: .regularExpression) != nil }
        }
    }

    @Test("Terminal, clicked: Deny keeps a program's copy off the clipboard and Allow lets the next one on; Split gaps and Option as Meta reach the window")
    @MainActor func terminalPageClicked() async throws {
        try await Instance.with { app in
            let esc = "\u{1b}", bel = "\u{07}"
            // Programs may not write the clipboard (core `settings_index/table/terminal.rs:100`; GPUI refuses the write,
            // `terminal_view/pane_bar.rs:74`). The marker after the sequence is on the screen twice once `cat` has written it back.
            await app.reveal("clipboard-write")
            await app.pressAlong("clipboard-write", at: Self.rightSegment) { await app.setting("clipboard_write") == #"string("deny")"# }
            _ = await app.waitForConfig { $0.range(of: #""clipboard_write"\s*:\s*"deny""#, options: .regularExpression) != nil }
            await app.emit("\(esc)]52;c;aGk=\(bel)denied-copy")
            await app.until("screen") { $0.components(separatedBy: "denied-copy").count - 1 >= 2 }
            app.expect(await app.reply("pasteboard") == "pasteboard empty", "the refused copy reached nothing")
            await app.pressAlong("clipboard-write", at: Self.leftSegment) { await app.setting("clipboard_write") == #"string("allow")"# }
            await app.emit("\(esc)]52;c;b2s=\(bel)allowed-copy")
            await app.until("pasteboard") { $0 == "pasteboard \"ok\"" }

            // Split gaps (`terminal.rs:29`; GPUI's panes read it, `window/panes/render.rs:64`).
            let gaps = Instance.value("gaps", in: await app.reply("state")) ?? "?"
            await app.reveal("split-gaps")
            _ = await Self.press(app, "split-gaps")
            await app.until("state") { Instance.value("gaps", in: $0) != gaps }
            _ = await app.waitForConfig { $0.range(of: #""split_gaps"\s*:\s*(true|false)"#, options: .regularExpression) != nil }

            // Option as Meta → Both (`terminal.rs:32`): the terminal's view types Meta with either Option.
            await app.reveal("meta")
            await app.pressAlong("meta", at: [(0.92, 0.5), (0.85, 0.5), (0.96, 0.5), (0.8, 0.5)]) { await app.setting("option_as_meta") == #"string("both")"# }
            await app.until("state") { Instance.value("meta", in: $0) == "both" }
            _ = await app.waitForConfig { $0.range(of: #""option_as_meta"\s*:\s*"both""#, options: .regularExpression) != nil }
        }
    }

    /// Where a two-segment control's second or first segment lies, nearest first.
    static let rightSegment: [(Double, Double)] = [(0.85, 0.5), (0.75, 0.5), (0.93, 0.5), (0.65, 0.5)]
    static let leftSegment: [(Double, Double)] = [(0.15, 0.5), (0.25, 0.5), (0.07, 0.5), (0.35, 0.5)]
}

extension Instance {
    /// Settings opened at row `id` (`settings-reveal`, schema rows) and its `settings-rows` line once it stands still.
    @discardableResult
    func reveal(_ id: String) async -> String {
        await send(["settings-reveal \(id)"])
        return await settledRow(id)
    }

    /// Clicks the Settings control recorded as `id` at the points given (fractions of its frame, `settings-press <id> at x y`),
    /// one at a time, until `done` holds. A segmented control's segments are as wide as their words, so the one wanted is
    /// found as an eye finds it, by where it is; each try is a real click on the control. Fails when none gave it.
    @discardableResult
    func pressAlong(_ id: String, at points: [(Double, Double)], until done: () async -> Bool) async -> Bool {
        if await done() { return true }
        for (x, y) in points {
            let said = await reply("settings-press \(id) at \(x) \(y)")
            if said.hasPrefix("error") {
                fail("settings-press \(id): \(said)")
                return false
            }
            if await done() { return true }
        }
        fail("no click along \(id) at \(points) gave what was wanted")
        return false
    }

    /// The text of a file of the default profile's own (`project-prefs.json`, `trusted-projects.json`): in the data folder, or
    /// where `configFile` would find it; empty while it is not there.
    func profileText(_ name: String) -> String {
        if let text = try? String(contentsOf: scratch.data.appendingPathComponent(name), encoding: .utf8) { return text }
        guard let walker = FileManager.default.enumerator(at: scratch.data, includingPropertiesForKeys: nil) else { return "" }
        for case let url as URL in walker where url.lastPathComponent == name && url.pathComponents.count - scratch.data.pathComponents.count <= 3 {
            return (try? String(contentsOf: url, encoding: .utf8)) ?? ""
        }
        return ""
    }

    /// Waits (pausing by the app's own `delay`, as `waitForConfig` does) until the profile's file `name` satisfies `done`.
    @discardableResult
    func waitForProfileText(_ name: String, timeout: TimeInterval = 15, _ done: (String) -> Bool) async -> String {
        let start = Date()
        var text = profileText(name)
        while !done(text), Date().timeIntervalSince(start) < timeout {
            await send(["delay 200"])
            text = profileText(name)
        }
        expect(done(text), "\(name) never said what was expected; it is:\n\(text)")
        return text
    }
}
