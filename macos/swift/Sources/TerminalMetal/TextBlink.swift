// Blinking text (SGR 5 and 6): one steady clock, shared by every terminal, that fades the glyphs of blinking cells
// in its off half (`render/blink.rs`). The phase is a function of the time, not state on the rows, so a flip
// rebuilds and shapes nothing: the glyphs carry a flag and the shader picks the colour. Rapid blink uses the same
// clock. One one-shot timer arms the next flip, only while a visible row blinks, the window is key and Reduce
// Motion is off; otherwise blinking text is steady.

import AppKit

enum TextBlinkClock {
    /// Each half of a cycle: 500 ms on, 500 ms faded, a second in all.
    static let phase = Duration.milliseconds(500)
    /// Every terminal counts from the same start, so panes blink together.
    static let start = ContinuousClock.now

    static func isFaded(at now: ContinuousClock.Instant = .now) -> Bool {
        Int((now - start) / phase) % 2 == 1
    }

    /// How long until the clock turns to its other half.
    static func untilFlip(from now: ContinuousClock.Instant = .now) -> Duration {
        let elapsed = now - start
        let into = elapsed - phase * Int(elapsed / phase)
        return max(phase - into, .milliseconds(1))
    }
}

extension MetalTerminalView {
    /// After each frame and when the window or Reduce Motion changes: run the clock while it is wanted.
    func updateTextBlink() {
        guard renderer.hasBlinkingText, window?.isKeyWindow == true, !reduceMotion else {
            textBlinkItem?.cancel()
            textBlinkItem = nil
            if renderer.setBlinkFadedChanges(false) { requestRedraw() }
            return
        }
        if renderer.setBlinkFadedChanges(TextBlinkClock.isFaded()) { requestRedraw() }
        guard textBlinkItem == nil else { return }
        let item = DispatchWorkItem { [weak self] in
            MainActor.assumeIsolated {
                guard let self else { return }
                self.textBlinkItem = nil
                self.updateTextBlink()
            }
        }
        textBlinkItem = item
        DispatchQueue.main.asyncAfter(deadline: .now() + TextBlinkClock.untilFlip().dispatchInterval, execute: item)
    }
}
