import Foundation
import Testing

/// A window's new size reaches the shell once. The pane area used to animate its frames from the old layout to the new one, and the
/// terminal view sized the program at every step of that motion, from a few columns up: a shell was told 30 times in half a second
/// (GPUI lays the panes out at the new size at once), and zsh redrew its prompt over its own text at each (the launch of a hosted
/// terminal lost its last line to it: `RestartedShellTests/restartedHostedShellKeepsItsText`).
extension EndToEnd {
    /// How many times the shell printed its mark: it prints over the prompt's row as well as on rows of its own.
    private static func marks(_ screen: String) -> Int { screen.components(separatedBy: "WINCHED").count - 1 }

    @Test("a new window size reaches the shell as one or two sizes, never as the steps of an animation")
    @MainActor func windowResizeSizesTheShellOnce() async throws {
        try await Instance.with(shell: .zsh) { app in
            await app.until("keys") { $0.contains("focus=terminal") }
            // zsh prints a mark each time it is told its window changed.
            await app.send(["text trap 'print WINCHED' WINCH", "key return", "text clear", "key return"])
            await app.send(["delay 500"])
            await app.send(["size 900x500", "frames 2", "delay 1200"])
            let small = Self.marks(await app.reply("screen"))
            app.expect(small >= 1 && small <= 2, "the shell was told \(small) times of a change to 900x500")
            await app.send(["size 1600x900", "frames 2", "delay 1200"])
            let all = Self.marks(await app.reply("screen"))
            app.expect(all >= 2 && all <= 4, "the shell was told \(all) times of two changes")
        }
    }
}
