import Foundation
import Testing

/// A window's new size reaches the shell once. The pane area used to animate its frames from the old layout to the new one, and the
/// terminal view sized the program at every step of that motion, from a few columns up: a shell was told 30 times in half a second
/// (GPUI lays the panes out at the new size at once), and zsh redrew its prompt over its own text at each (the launch of a hosted
/// terminal lost its last line to it: `RestartedShellTests/restartedHostedShellKeepsItsText`).
extension EndToEnd {
    /// How many times the shell has been told its window changed: zsh counts them in `n` and says it when asked (`echo CNT=$n`),
    /// a short line however narrow the terminal is and whatever a prompt's redraw does to the rows above it.
    @MainActor private static func told(_ app: Instance) async -> Int? {
        await app.send(["text echo CNT=$n", "key return"])
        let screen = await app.screen { $0.split(separator: "\n").contains { $0.trimmingCharacters(in: .whitespaces).range(of: #"^CNT=\d+$"#, options: .regularExpression) != nil } }
        return screen.split(separator: "\n").map { $0.trimmingCharacters(in: .whitespaces) }.last { $0.range(of: #"^CNT=\d+$"#, options: .regularExpression) != nil }
            .flatMap { Int($0.dropFirst(4)) }
    }

    @Test("a new window size reaches the shell as one or two sizes, never as the steps of an animation")
    @MainActor func windowResizeSizesTheShellOnce() async throws {
        try await Instance.with(shell: .zsh) { app in
            await app.until("keys") { $0.contains("focus=terminal") }
            await app.send(["text n=0; trap 'n=$((n+1))' WINCH", "key return", "delay 500"])
            // Two sizes that both differ from the suite's 1600 x 900 (the start) and from each other, and both fit a small screen: a
            // size the window already has tells the shell nothing.
            await app.send(["size 900x500", "frames 2", "delay 1200"])
            let small = await Self.told(app)
            app.expect(small.map { $0 >= 1 && $0 <= 2 } == true, "the shell was told \(small.map(String.init) ?? "?") times of a change to 900x500")
            await app.send(["size 1200x700", "frames 2", "delay 1200"])
            let all = await Self.told(app)
            app.expect(all.map { $0 >= 2 && $0 <= 4 } == true, "the shell was told \(all.map(String.init) ?? "?") times of two changes")
        }
    }
}
