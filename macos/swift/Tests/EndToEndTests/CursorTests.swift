import Foundation
import Testing

/// The terminal's cursor as its view draws it (T03), read with `cursor [terminal]`: what `TerminalScene` builds the cursor's row
/// from (the engine's DECSCUSR shape and blink, the program's OSC 12 colour over the theme's, whether the view has the keys,
/// whether the program ended). `cat` writes each sequence back, so the terminal reads it as a program's.
extension EndToEnd {
    @discardableResult
    @MainActor static func cursor(_ app: Instance, _ terminal: Int? = nil, _ done: (String) -> Bool) async -> String {
        await app.until(terminal.map { "cursor \($0)" } ?? "cursor") { $0.hasPrefix("cursor drawn=") && done($0) }
    }

    // GPUI: `terminal_view/render/cursor.rs:71` (`cursor_rect`: the shape while focused, the cell's outline when not), `:64`
    // (no cursor once the program exited), DECSCUSR and OSC 12/112 in the engine (`../repo-docs/docs/escape-sequences.md`).
    @Test("DECSCUSR's shapes and blink, OSC 12's colour and its reset, the outline in the pane without the keys, no cursor after exit")
    @MainActor func cursorShapesColourFocusAndExit() async throws {
        try await Instance.with { app in
            let start = await Self.cursor(app) { _ in true }
            let theme = Instance.value("colour", in: start) ?? "?"

            await app.emit("\u{1b}[4 q")
            await Self.cursor(app) { $0.contains("shape=underline") && $0.contains("blinking=false") }
            await app.emit("\u{1b}[5 q")
            await Self.cursor(app) { $0.contains("shape=beam") && $0.contains("blinking=true") }
            await app.emit("\u{1b}[2 q")
            await Self.cursor(app) { $0.contains("shape=block") && $0.contains("blinking=false") }

            // A program's cursor colour, then its reset to the theme's (ST-terminated: no bell).
            await app.emit("\u{1b}]12;#ff8800\u{1b}\\")
            await Self.cursor(app) { Instance.value("colour", in: $0) == "#ff8800" }
            await app.emit("\u{1b}]112\u{1b}\\")
            await Self.cursor(app) { Instance.value("colour", in: $0) == theme }

            // A split: the pane that was left draws the cell's outline; the new one, with the keys, its own shape.
            guard let first = (await app.rows()).selected else { return }
            await app.send(["key cmd-d"])
            let panes = Panes(await app.until("panes") { Panes($0).ids.count == 2 })
            guard let focused = panes.focused, focused != first else { return app.fail("the new pane has the keys:\n\(panes)") }
            let left = await Self.cursor(app, first) { $0.contains("focused=false") }
            app.expect(left, contains: "drawn=outline")
            let shown = await Self.cursor(app, focused) { $0.contains("focused=true") }
            app.expect(!shown.contains("drawn=outline") && !shown.contains("drawn=none"), "the pane with the keys draws its shape:\n\(shown)")

            // `cat` reads the end of its input and exits: nothing reads keys there any more, so no cursor is drawn.
            await app.send(["type \u{4}"])
            let ended = await Self.cursor(app, focused) { $0.contains("hidden=true") }
            app.expect(ended, contains: "drawn=none")
        }
    }
}
