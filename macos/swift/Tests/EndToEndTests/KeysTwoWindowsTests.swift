import Foundation
import Testing

/// Keys sent to one of two workspace windows, typed text and a key equivalent. ⌘← is the terminal's own (the line editor's:
/// it never goes through a menu), so it is asked of the views of the window it was sent to. The application hands a key to
/// the window it holds for key itself, which in a test instance is the first one: both kinds were typed into the first
/// window's terminal whichever window's control file sent them, until the driver made that window hand the key on.
extension EndToEnd {
    @Test("⌘← sent to each of two windows moves the cursor in that window's own shell, and not in the other's")
    @MainActor func lineEditorKeysInEachWindow() async throws {
        try await Instance.with(shell: .zsh) { app in
            await app.send(["new-window"])
            await app.until("sessions") { $0.split(separator: "\n").count == 2 }
            // A short prompt in each shell: a host name's long prompt wraps a typed line in a narrow window (the second window
            // is some 35 columns wide on a small display), and a line wrapped on two rows is not one string on the screen.
            for window in [1, 2] {
                await app.send([#"text PS1='$ '"#, "key return"], window: window)
                await app.until("screen", window: window) { $0.contains("\n$ ") || $0.hasPrefix("$ ") }
            }
            for window in [1, 2] {
                await app.send(["text echo w\(window)"], window: window)
                await app.until("screen", window: window) { $0.contains("echo w\(window)") }
            }
            // The window opened last first, then the one behind it: ^A, then a letter at the line's start.
            for window in [2, 1] {
                await app.send(["key cmd-left", "text x", "key return"], window: window)
                let screen = await app.until("screen", window: window) { $0.contains("xecho w\(window)") }
                app.expect(screen, contains: "xecho w\(window)")
            }
            // Neither shell got anything meant for the other: what was typed for one window is on that window's screen only.
            for (window, other) in [(1, 2), (2, 1)] {
                let screen = await app.reply("screen", window: window)
                app.expect(!screen.contains("w\(other)"), "nothing typed for window \(other) is in window \(window):\n\(screen)")
            }
        }
    }
}
