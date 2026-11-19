import Foundation
import Testing

/// A key of a feature that is off. The keymap's row for ⌥⌘S (Stash Prompt) is tagged with the Notes feature: off, the GPUI
/// app binds no handler for it (`window/feature_host.rs` `bind_features` skips a feature the mask leaves out) and hides its
/// menu item (`FeatureMask::hidden_actions`), so the key is nobody's. The terminal does not take it either: some row of the
/// table uses the key, so it is not ⌘S's stand-in (`platform_command`).
extension EndToEnd {
    @Test("⌥⌘S with Notes off makes no note, shows no tab and types nothing; with Notes on the same key makes the note")
    @MainActor func stashKeyWithNotesOff() async throws {
        try await Instance.with { app in
            // Notes is behind its flag, which a release build leaves off: the panel has no Notes tab.
            let tabs = await app.reply("panel-tabs")
            app.expect(!tabs.contains("tab notes"), "no Notes tab while the feature is off:\n\(tabs)")
            await app.send(["text stash-me", "key return"])
            let screen = await app.screen(showing: "stash-me")
            let lines = screen.split(separator: "\n", omittingEmptySubsequences: false).map(String.init)
            guard let row = lines.firstIndex(where: { $0.hasPrefix("stash-me") }) else { return app.fail("on screen:\n\(screen)") }
            let shown = Instance.value("tab", in: await app.reply("state")) ?? ""

            let pressed = (await app.send(["select-cells \(row) 0 \(row) 7", "key alt-cmd-s"])).last ?? ""
            app.expect(!pressed.contains("stashPrompt"), "the key ran no action: \(pressed)")
            // What is typed next arrives alone: the key wrote nothing to the program.
            await app.send(["text after-the-key", "key return"])
            let after = await app.screen(showing: "after-the-key")
            let typed = after.split(separator: "\n").map(String.init).filter { $0.contains("after-the-key") }
            app.expect(typed.allSatisfy { $0.hasPrefix("after-the-key") }, "nothing was typed before it:\n\(after)")
            let notes = await app.reply("notes")
            app.expect(notes, contains: "notes=[]")
            app.expect(Instance.value("tab", in: await app.reply("state")) == shown, "the panel's tab did not change from \(shown)")

            // The control: with the feature on, the same selection and the same key make the note and show Notes.
            await app.flags(["notes_and_tasks": true])
            await app.until("panel-tabs") { $0.contains("tab notes") }
            await app.send(["select-cells \(row) 0 \(row) 7", "key alt-cmd-s"])
            await app.until("notes") { $0.contains("stash-me") }
            await app.until("state") { $0.contains("tab=notes") }
        }
    }
}
