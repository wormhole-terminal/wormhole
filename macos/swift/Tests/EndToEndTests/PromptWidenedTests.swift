import Foundation
import Testing

/// A window widened while zsh's prompt is wrapped. KNOWN AND UNPROVEN, kept here so it is counted and not forgotten (`TO-VERIFY.md`,
/// "round 14", `restartedHostedShellKeepsItsText` with a 62-character prompt).
///
/// A pane 35 columns wide holds a 62-character prompt on two rows. Widened to 103 columns the engine reflows those soft-wrapped rows
/// into one (`wormhole-vt`, `grid/resize.rs`: no case for a prompt the shell is about to redraw), and zsh, told its window changed,
/// moves up the rows it believes its prompt takes and clears down: the output line above the prompt goes. The GPUI app does the same
/// with the same engine: `Pty::resize` (`crates/wormhole-term/src/handle/size.rs`) reflows the screen and then sets the PTY's window
/// size, and the view calls it at once for the size the pane has (`terminal_view/scaling/live.rs`), the same order and no delay.
/// Not in this repository's hands (the engine is its own), so the scenario is disabled, not weakened.
extension EndToEnd {
    @Test(
        "an output line above a wrapped prompt is still there when the window is widened",
        .disabled("the engine reflows a wrapped prompt zsh then redraws over the line above: wormhole-vt's resize, see TO-VERIFY.md round 14"))
    @MainActor func widenedWhileThePromptWraps() async throws {
        try await Instance.with(shell: .zsh) { app in
            await app.until("keys") { $0.contains("focus=terminal") }
            // A pane some 35 columns wide with both columns open, and a prompt of the length a runner's host name makes.
            await app.send(["size 944x600", "frames 2"])
            await app.until("geometry") { $0.contains("content=(944.0, 600.0)") }
            await app.send(["text PS1='\(Instance.longPrompt)% '", "key return", "text echo above-the-prompt", "key return"])
            await app.screen { $0.split(separator: "\n").contains { $0.hasPrefix("above-the-prompt") } }

            await app.send(["size 1600x900", "frames 2", "delay 1000"])
            let widened = await app.reply("screen")
            app.expect(widened.split(separator: "\n").contains { $0.hasPrefix("above-the-prompt") }, "the line above the prompt is still there:\n\(widened)")
        }
    }
}
