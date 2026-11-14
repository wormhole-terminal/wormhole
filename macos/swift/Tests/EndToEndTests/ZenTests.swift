import Foundation
import Testing

/// Zen mode and the first run's presets, read from what the window drew (`marked`: the status bar, the panel's tabs) and the
/// title row it was given (`title-menu`'s head line), not from the switches the commands set.
extension EndToEnd {
    /// Whether the window drew an element of each id now.
    static func drawn(_ said: String, _ ids: [String]) -> [Bool] {
        let all = Set(frames(said).map(\.id))
        return ids.map(all.contains)
    }

    @Test("Zen hides the status bar and the panel and leaves the title row the name and the splits; off, they come back")
    @MainActor func zenHidesTheChrome() async throws {
        try await Instance.with { app in
            await app.send(["set status_bar=true"])
            // `panel` is View ▸ Panel, a toggle.
            if !(await app.reply("state")).contains("panel=true") { await app.send(["panel"]) }
            await app.until("state") { $0.contains("panel=true") }
            await app.send(["inspect on"])
            let ids = ["status.bar", "icon.tab"]
            await app.until("marked") { Self.drawn($0, ids) == [true, true] }
            let before = await app.reply("title-menu")

            // Zen: the name, a space and the splits in the title row; no status bar, minimap, dividers or panel
            // (wormhole-ffi focus.rs:295, the core's window preset).
            app.expect(await app.reply("zen on"), contains: "zen on")
            await app.until("marked") { Self.drawn($0, ids) == [false, false] }
            // The person's own switch is kept while Zen draws no panel, so it is back as it was after (`window/layout`,
            // `zen_tests.rs`: "the panel is as it was after"); what the window draws is what `marked` read above.
            app.expect(await app.reply("state"), contains: "panel=true")
            app.expect((await app.reply("title-menu")).hasPrefix("title-menu row=terminal,space,splits"), "the title row under Zen")

            app.expect(await app.reply("zen off"), contains: "zen off")
            await app.until("marked") { Self.drawn($0, ids) == [true, true] }
            let after = await app.reply("title-menu")
            app.expect(after.split(separator: "\n").first == before.split(separator: "\n").first, "the title row as it was:\n\(before)\n\(after)")
        }
    }

    @Test("the first run's Everything turns on the status bar and opens the panel; Start over puts Balanced back")
    @MainActor func tailorPresets() async throws {
        try await Instance.with { app in
            _ = await Self.onboarding(app, at: "look")
            await app.send(["inspect on"])
            await app.until("marked") { Self.frames($0).contains { $0.id == "sidebar.row" } }
            // Every answer is applied as it is given (core onboarding/tailor.rs:7); Everything is the third (config/presets.rs:162).
            app.expect(await app.reply("onboarding-tailor 3"), contains: "onboarding-tailor 3")
            _ = await app.waitForConfig { $0.range(of: #""status_bar"\s*:\s*true"#, options: .regularExpression) != nil }
            await app.until("marked") { Self.drawn($0, ["status.bar"]) == [true] }
            await app.until("state") { $0.contains("panel=true") }
            // Start over: Balanced, as the defaults are (presets.rs:154), so the status bar goes again.
            await app.send(["onboarding-tailor over"])
            _ = await app.waitForConfig { $0.range(of: #""status_bar"\s*:\s*false"#, options: .regularExpression) != nil }
            await app.until("marked") { Self.drawn($0, ["status.bar"]) == [false] }
        }
    }
}
