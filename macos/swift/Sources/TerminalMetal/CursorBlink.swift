// The cursor's blink (and, through `updateTextBlink`, the text's): only when the program asked for it (DECSCUSR), in the key window, and with Reduce Motion
// off. One one-shot timer arms the next flip and nothing else runs, so a steady cursor costs no wakeups.
// Typing shows the cursor at once and keeps it on for a full phase, as in any text field.

import AppKit

extension MetalTerminalView {
    var reduceMotion: Bool { NSWorkspace.shared.accessibilityDisplayShouldReduceMotion }

    /// Typing: the cursor is on now and the next flip is a whole phase away.
    func holdCursor() {
        guard renderer.scene.cursor?.blinking == true else { return }
        blinkItem?.cancel()
        blinkItem = nil
        renderer.setCursorBlinkOn(true)
        updateBlink()
    }

    /// `focused` is given while the responder chain is changing (the view is not yet, or no longer, the first responder).
    func updateBlink(focused: Bool? = nil) {
        updateTextBlink()
        let blinking = renderer.scene.cursor?.blinking == true && (focused ?? hasFocus) && blinkInterval != nil && !reduceMotion
        if blinking {
            guard blinkItem == nil, let interval = blinkInterval else { return }
            let item = DispatchWorkItem { [weak self] in
                MainActor.assumeIsolated {
                    guard let self else { return }
                    self.blinkItem = nil
                    self.statistics.blinkFires += 1
                    self.renderer.setCursorBlinkOn(!self.renderer.scene.cursorOn)
                    self.requestRedraw()
                }
            }
            blinkItem = item
            DispatchQueue.main.asyncAfter(deadline: .now() + interval.dispatchInterval, execute: item)
        } else if blinkItem != nil || !renderer.scene.cursorOn {
            blinkItem?.cancel()
            blinkItem = nil
            renderer.setCursorBlinkOn(true)
            requestRedraw()
        }
    }
}

extension Duration {
    var dispatchInterval: DispatchTimeInterval {
        .nanoseconds(Int(components.seconds) * 1_000_000_000 + Int(components.attoseconds / 1_000_000_000))
    }
}
