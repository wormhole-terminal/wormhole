import Foundation
import Testing

/// What VoiceOver gets of the two bars a terminal shows over its pane (`a11y`: the tree, one element a line as `role "name"`, a
/// child indented under its parent): the exited bar is one labelled group holding its words and its two buttons, and the sticky
/// warning is a button named by its words with its two small buttons named. The words are the bar's own (`exited-words`).
extension EndToEnd {
    /// The lines of the dump under the line `parent` names: those indented deeper than it, until one that is not.
    private static func children(of parent: String, in tree: String) -> [String] {
        let lines = tree.split(separator: "\n", omittingEmptySubsequences: false).map(String.init)
        guard let at = lines.firstIndex(where: { $0.trimmingCharacters(in: .whitespaces).hasPrefix(parent) }) else { return [] }
        let depth = lines[at].prefix { $0 == " " }.count
        return lines[(at + 1)...].prefix { $0.prefix { $0 == " " }.count > depth }.map { $0.trimmingCharacters(in: .whitespaces) }
    }

    @Test("the exited bar is a group named by its words with a Restart and a Close terminal button; the sticky warning's buttons are named")
    @MainActor func barsAreSpoken() async throws {
        try await Instance.with(shell: .zsh) { app in
            await app.send(["notice host-fallback"])
            await app.until("notice chip") { $0.contains("ends when") }
            let chip = await app.reply("a11y")
            let parts = Self.children(of: "Button \"This terminal ends when Wormhole quits\"", in: chip)
            app.expect(parts.contains { $0.hasPrefix("StaticText") && $0.contains("This terminal ends when Wormhole quits") }, "the chip says its words:\n\(chip)")
            app.expect(parts.contains { $0.hasPrefix("Button \"More about this warning\"") }, "its ⓘ is a named button:\n\(parts)")
            app.expect(parts.contains { $0.hasPrefix("Button \"Close\"") }, "its × is a named button:\n\(parts)")
            await app.send(["notice-click close"])

            await app.shell("exit")
            let words = await app.until("exited-words") { $0 != "exited-words none" }
            guard let said = words.split(separator: " ", maxSplits: 1).last.map({ String($0).trimmingCharacters(in: CharacterSet(charactersIn: "\"\n ")) }) else { return app.fail("the bar's words:\n\(words)") }
            let bar = await app.reply("a11y")
            let inside = Self.children(of: "Group \"\(said)\"", in: bar)
            app.expect(!inside.isEmpty, "the bar is a group named by its words (\(said)):\n\(bar)")
            app.expect(inside.contains { $0.hasPrefix("StaticText") && $0.contains(said) }, "the group holds its words:\n\(inside)")
            let buttons = inside.filter { $0.hasPrefix("Button") }
            app.expect(buttons.map { $0.split(separator: "\"").dropFirst().first.map(String.init) ?? "" } == ["Restart", "Close terminal"], "Restart, then Close terminal:\n\(buttons)")
        }
    }
}
