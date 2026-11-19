import Foundation
import Testing

/// A window widened while zsh's prompt is wrapped.
///
/// A pane 35 columns wide holds a 62-character prompt on two rows. zsh remembers that its cursor is one row under the prompt's
/// first; told of 103 columns it goes up one row, erases to the end of the screen and draws the prompt again. The engine used to
/// join the two rows into one before that, so zsh's one row up was the output line above the prompt, and it went. The engine now
/// leaves the rows of the cursor's line as they are when the screen gets wider (`wormhole-vt`, `grid/resize.rs`, with zsh's and
/// bash's bytes in its `resize.vtt`). The GPUI app has the same engine and tells it the size in the same order.
///
/// Still open there: a prompt of several lines whose upper line wraps. That line is not the cursor's, it is joined, and zsh counts
/// its rows too (`TO-VERIFY.md`, round 16).
extension EndToEnd {
    @Test("an output line above a wrapped prompt is still there when the window is widened")
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
