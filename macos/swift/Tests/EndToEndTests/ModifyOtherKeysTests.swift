import Foundation
import Testing

/// xterm's modifyOtherKeys (T07): a program asks for it with `CSI > 4 ; Pv m` and a key pressed with a modifier then reaches it
/// as `CSI 27 ; mods ; code ~`. `cat` is the program: what a key sends is echoed by the line discipline (`^[` for ESC), so the
/// screen shows the bytes the key became.
extension EndToEnd {
    // The engine's rules (`wormhole-vt` `docs/input.md` "modifyOtherKeys"): level 2 makes every modified key a sequence, level 1
    // only those without a control code (Ctrl+I stays Tab, Ctrl+, is a sequence), and off sends what the key types.
    @Test("modifyOtherKeys level 2 sends Ctrl+I as CSI 27;5;105~, level 1 keeps it Tab but sends Ctrl+, as CSI 27;5;44~, off neither")
    @MainActor func modifyOtherKeysLevels() async throws {
        try await Instance.with { app in
            await app.emit("\u{1b}[>4;2m")
            await app.send(["wait-idle 2000", "key ctrl-i"])
            await app.screen(showing: "^[[27;5;105~")

            await app.discardLine()
            await app.emit("\u{1b}[>4;1m")
            await app.send(["wait-idle 2000", "key ctrl-i", "key ctrl-,"])
            await app.screen(showing: "^[[27;5;44~")
            // The line discipline erased level 2's sequence with the line (^U), so one on screen now would be level 1's.
            app.expect(await app.count("^[[27;5;105~") == 0, "at level 1 Ctrl+I is Tab again")

            await app.discardLine()
            await app.emit("\u{1b}[>4m")
            await app.send(["wait-idle 2000", "key ctrl-,", "text off-mark"])
            await app.screen(showing: "off-mark")
            app.expect(await app.count("^[[27;5;44~") == 0, "with the mode off Ctrl+, is no sequence")
        }
    }
}
