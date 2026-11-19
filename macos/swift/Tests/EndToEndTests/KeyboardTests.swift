import Foundation
import Testing

/// The window without a pointer (F-70): F6 between the areas, arrows on the panel's tab bar, ⌘F from the list, Esc closing what
/// covers the terminal, and the terminal's text as the accessibility tree gives it to VoiceOver. `key` answers with where the
/// keys are after it (`focus=<area>[ <item>][ ring][ +find][ +cover]`, `WindowFocus.detail`).
extension EndToEnd {
    /// What a `key` reply says the focus is (`key f6 -> focusNextArea focus=sidebar`).
    private static func focus(_ reply: String) -> String {
        reply.range(of: "focus=").map { String(reply[$0.upperBound...]) } ?? ""
    }

    @Test("F6 goes round the areas back to the terminal, arrows walk the panel's tabs, ⌘F from the list opens Find, Esc closes a diff")
    @MainActor func keyboardAreas() async throws {
        try await Instance.with { app in
            let repo = try app.scratch.repository(files: ["README.md": "one\n"])
            try app.scratch.write(repo, "README.md", "one\ntwo\n")
            await app.send(["new \(repo.path)"])
            await app.live(2)
            if !(await app.reply("state")).contains("panel=true") { await app.send(["panel"]) }
            await app.until("changes") { $0.contains("files=1") }

            var seen: [String] = []
            for _ in 0..<10 {
                let area = Self.focus(await app.reply("key f6"))
                seen.append(area)
                if area.hasPrefix("terminal") { break }
            }
            app.expect(seen.last?.hasPrefix("terminal") == true, "F6 came back to the terminal: \(seen)")
            app.expect(seen.contains { $0.hasPrefix("sidebar") }, "F6 reached the list: \(seen)")
            app.expect(seen.contains { $0.hasPrefix("panel-tabs") }, "F6 reached the panel's tabs: \(seen)")

            // On the panel's tab bar ←/→ step between the tabs.
            let tabs = await Self.press("f6", in: app, until: "panel-tabs")
            let stepped = Self.focus(await app.reply("key right"))
            app.expect(stepped.hasPrefix("panel-tabs") && stepped != tabs, "→ moved along the tabs: \(tabs) then \(stepped)")

            // ⌘F with the list focused opens the terminal's Find (the sidebar has no search of its own); Esc on the terminal closes it.
            await Self.press("f6", in: app, until: "sidebar")
            app.expect(Self.focus(await app.reply("key cmd-f")), contains: "+find")
            await Self.press("f6", in: app, until: "terminal")
            let closed = Self.focus(await app.reply("key esc"))
            app.expect(!closed.contains("+find"), "Esc closed Find: \(closed)")

            // A diff over the terminal, the keys on the terminal: Esc takes it away.
            await app.send(["diff 1"])
            await app.until("state") { !$0.contains("cover=none") }
            await app.send(["key esc"])
            await app.until("state") { $0.contains("cover=none") }
        }
    }

    /// Presses `key` until the focus is in the area `prefix` names (ten times at most); the focus it ends on.
    @discardableResult
    @MainActor private static func press(_ key: String, in app: Instance, until prefix: String) async -> String {
        var area = ""
        for _ in 0..<10 {
            area = focus(await app.reply("key \(key)"))
            if area.hasPrefix(prefix) { return area }
        }
        app.fail("the keys never reached \(prefix); the last focus was \(area)")
        return area
    }

    @Test("the terminal's text area gives VoiceOver the screen's text")
    @MainActor func terminalAccessibility() async throws {
        try await Instance.with { app in
            await app.emit("read-aloud")
            await app.screen { $0.contains("read-aloud") }
            let tree = await app.reply("a11y")
            let area = tree.split(separator: "\n").first { $0.trimmingCharacters(in: .whitespaces).hasPrefix("TextArea") }
            app.expect(area?.contains("read-aloud") == true, "the text area's value is the screen's text:\n\(tree)")
        }
    }
}
