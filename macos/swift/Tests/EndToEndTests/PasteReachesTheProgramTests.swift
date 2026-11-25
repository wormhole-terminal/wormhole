import Foundation
import Testing

/// What ⌘V puts into a program (T04, T07): the instance's clipboard (`pasteboard-set`), the key, and the bytes the program
/// reads, which `cat`'s line discipline shows on the screen as it echoes them (ESC as `^[`, a line break as a new row). A program
/// that asked for bracketed paste (2004) gets the text between `CSI 200 ~` and `CSI 201 ~`, and nothing in the text can end the
/// bracket early; one that did not gets the text, with its line breaks as Return.
extension EndToEnd {
    /// The screen as one line of text, the rows joined and their padding dropped.
    private static func flat(_ screen: String) -> String {
        screen.split(separator: "\n", omittingEmptySubsequences: false).map { $0.trimmingCharacters(in: .whitespaces) }.joined(separator: "|")
    }

    @Test("a bracketed paste is wrapped, a program that did not ask gets the text; line breaks are Returns; nothing in the text ends the bracket")
    @MainActor func pasteIsBracketedAndCleaned() async throws {
        try await Instance.with { app in
            // Not asked for: the text as it is, the break as a Return (the line discipline echoes it as a line), no markers.
            await app.send(["pasteboard-set plain one\\nplain two", "key cmd-v"])
            let plain = await app.screen { $0.contains("plain two") }
            app.expect(!plain.contains("^[[200~"), "no bracket for a program that did not ask:\n\(plain)")
            app.expect(plain.components(separatedBy: "plain one").count == 3, "the first line was read as a line, so it is there twice (typed echo and cat's answer):\n\(plain)")
            await app.discardLine()

            // Asked for: the bracket round the text.
            await app.emit("\u{1b}[?2004h")
            await app.send(["wait-idle 2000", "type \u{15}", "pasteboard-set bracketed", "key cmd-v"])
            let bracketed = await app.screen { $0.contains("^[[200~bracketed^[[201~") }
            app.expect(bracketed, contains: "^[[200~bracketed^[[201~")
            await app.discardLine()

            // ESC, an end marker in the text and C1 controls are dropped, so the text cannot close the bracket early (the
            // injection bracketed paste exists to stop); tab and other text pass.
            await app.send(["pasteboard-set before\\e[201~ rm -rf after\\e\u{9b}31m tail", "key cmd-v"])
            let cleaned = await app.screen { $0.contains("tail^[[201~") }
            let line = Self.flat(cleaned)
            app.expect(line.contains("^[[200~before[201~ rm -rf after31m tail^[[201~"), "ESC and C1 controls are gone, the rest is typed, one end marker at the very end:\n\(cleaned)")
            app.expect(line.components(separatedBy: "^[[201~").count == 2, "exactly one end marker:\n\(cleaned)")

            // Without the bracket nothing protects the program: ESC is passed (as xterm's paste does).
            await app.discardLine()
            await app.emit("\u{1b}[?2004l")
            await app.send(["wait-idle 2000", "type \u{15}", "pasteboard-set raw\\e[0m", "key cmd-v"])
            let raw = await app.screen { $0.contains("raw^[[0m") }
            app.expect(raw, contains: "raw^[[0m")
        }
    }
}
