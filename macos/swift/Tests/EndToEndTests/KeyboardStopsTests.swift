import Foundation
import Testing

/// The window without a pointer, past F6 (`KeyboardTests`): ⇥ and ⇧⇥ stop by stop (core `view/focus/tab.rs`), the title row's
/// buttons only with macOS's Keyboard navigation (`keyboard-navigation on|off`, which stands in for the system's switch), ↩ on
/// the stop the keys are on; the sidebar's type-ahead and arrows, and the arrows in the Changes list (core `view/focus/list.rs`).
/// Each `key` answers with where the keys are after it (`focus=<area>[ <item>][ ring]…`, `WindowFocus.detail`).
extension EndToEnd {
    /// Where a `key` reply says the keys are.
    private static func keysAt(_ reply: String) -> String {
        reply.range(of: "focus=").map { String(reply[$0.upperBound...]) } ?? ""
    }

    /// Presses `key` until the keys are in the area `prefix` names (ten times at most).
    @discardableResult
    @MainActor private static func stepKeys(_ key: String, in app: Instance, until prefix: String) async -> String {
        var area = ""
        for _ in 0..<10 {
            area = keysAt(await app.reply("key \(key)"))
            if area.hasPrefix(prefix) { return area }
        }
        app.fail("`key \(key)` never reached \(prefix); the last focus was \(area)")
        return area
    }

    @Test("⇥ and ⇧⇥ walk the stops in F6's order and wrap; the title row's buttons are stops only with Keyboard navigation on, and ↩ presses the one the keys are on")
    @MainActor func tabStops() async throws {
        try await Instance.with { app in
            let repo = try app.scratch.repository(files: ["a.txt": "a\n"])
            try app.scratch.write(repo, "a.txt", "a\nb\n")
            await app.terminal(in: repo)
            if !(await app.reply("state")).contains("panel=true") { await app.send(["panel"]) }
            await app.until("changes") { $0.contains("files=1") }
            await app.send(["keyboard-navigation off"])

            // From the list: the terminal's strip, then the terminal (`tab.rs:19`, F6's order: title row, sidebar, tabs,
            // terminal, panel tabs, panel lists, status bar).
            await Self.stepKeys("f6", in: app, until: "sidebar")
            app.expect(Self.keysAt(await app.reply("key tab")).hasPrefix("tabs"), "⇥ from the list goes to the terminal's strip")
            app.expect(Self.keysAt(await app.reply("key tab")).hasPrefix("terminal"), "⇥ from the strip goes to the terminal")

            // From the panel's tabs: its list, then round past the hidden status bar and the title row (no Keyboard navigation)
            // to the list; ⇧⇥ comes back to the panel's last stop.
            await Self.stepKeys("f6", in: app, until: "panel-tabs")
            app.expect(Self.keysAt(await app.reply("key tab")).hasPrefix("panel-content"), "⇥ from the panel's tabs goes into its list")
            app.expect(Self.keysAt(await app.reply("key tab")).hasPrefix("sidebar"), "⇥ wraps to the list: the title row is no stop without Keyboard navigation")
            app.expect(Self.keysAt(await app.reply("key shift-tab")).hasPrefix("panel-content"), "⇧⇥ goes back to the panel's list")

            // With Keyboard navigation: ⇧⇥ from the list enters the title row at its last button, then the first; ↩ presses it.
            await app.send(["keyboard-navigation on"])
            await Self.stepKeys("f6", in: app, until: "sidebar")
            app.expect(Self.keysAt(await app.reply("key shift-tab")), contains: "title-row new-terminal")
            app.expect(Self.keysAt(await app.reply("key shift-tab")), contains: "title-row toggle-sidebar")
            await app.send(["key return"])
            await app.until("state") { $0.contains("sidebar=false") }
            app.expect(Self.keysAt(await app.reply("keys")).hasPrefix("title-row"), "the keys stay on the button after the press")
            await app.send(["key return"])
            await app.until("state") { $0.contains("sidebar=true") }
            await app.send(["keyboard-navigation system"])
        }
    }

    @Test("in the sidebar a typed letter goes to the terminal named so and ↩ selects it; ↑ then ↩ selects the one above")
    @MainActor func sidebarTypeAheadAndArrows() async throws {
        try await Instance.with { app in
            await app.send(["key cmd-t", "key cmd-t"])
            guard let ids = await app.live(3) else { return }
            await app.send(["rename \(ids[0]) alpha", "rename \(ids[1]) beta", "rename \(ids[2]) gamma", "select \(ids[0])"])
            await app.until("rows") { Listing($0).row(ids[2])?.title == "gamma" && Listing($0).selected == ids[0] }

            // Type-ahead (`list.rs:136`): `g` is gamma; ↩ opens the terminal the keys are on.
            await Self.stepKeys("f6", in: app, until: "sidebar")
            await app.send(["text g", "key return"])
            await app.until("rows") { Listing($0).selected == ids[2] }

            // ↑ from gamma, the row the keys came back to, is beta (`list.rs:68`).
            await Self.stepKeys("f6", in: app, until: "sidebar")
            await app.send(["key up", "key return"])
            await app.until("rows") { Listing($0).selected == ids[1] }
        }
    }

    @Test("in the Changes list ↓ and ↩ open a file's diff, and with a diff open the arrows take it from file to file")
    @MainActor func changesListArrows() async throws {
        try await Instance.with { app in
            let repo = try app.scratch.repository(files: ["a.txt": "a\n", "b.txt": "b\n", "c.txt": "c\n"])
            for name in ["a.txt", "b.txt", "c.txt"] { try app.scratch.write(repo, name, "changed\n") }
            await app.send(["set changes_tree=false"])
            await app.terminal(in: repo)
            if !(await app.reply("state")).contains("panel=true") { await app.send(["panel"]) }
            let listed = await app.until("changes") { $0.contains("files=3") }
            let order = listed.split(separator: "\n").dropFirst().compactMap { $0.split(separator: " ").dropFirst().first.map(String.init) }
            app.expect(order.count == 3, "three files listed:\n\(listed)")

            // Into the list from its tab (↓ on the panel's tabs, `WindowFocus+Keys.swift`), ↓ to a file, ↩ opens it (`list.rs:31`).
            await Self.stepKeys("f6", in: app, until: "panel-tabs")
            app.expect(Self.keysAt(await app.reply("key down")).hasPrefix("panel-content"), "↓ on the tabs goes into the list")
            await app.send(["key down", "key return"])
            let opened = Readout(await app.until("diff-header") { Readout($0).first("diff-header")?.label != "none" })
            guard let first = opened.first("diff-header")?.label, let at = order.firstIndex(of: first), at + 1 < order.count else {
                return app.fail("a file of the list opened, not the last:\n\(opened.text)")
            }
            // With a diff open the keys carry it along (GPUI `features/git/changes/list.rs`).
            await app.send(["key down"])
            await app.until("diff-header") { Readout($0).first("diff-header")?.label == order[at + 1] }
            await app.send(["key up"])
            await app.until("diff-header") { Readout($0).first("diff-header")?.label == first }
        }
    }
}
