// Keyboard focus and the system settings the view follows: a window becoming key or not, the first responder,
// the scroller style, Reduce Motion. The cursor is hollow without focus, a program that asked for focus events
// (DECSET 1004) hears about it, and composing text is dropped when the keyboard leaves.

import AppKit

extension MetalTerminalView {
    @objc func keyStateChanged(_ note: Notification) {
        renderer.setFocused(window?.isKeyWindow ?? false)
        reportOnScreen()
        if window?.isKeyWindow != true { endComposition() }
        reportFocus()
        updateBlink()
        requestRedraw()
    }

    @objc func scrollerStyleChanged(_ note: Notification) {
        scroller.applyStyle()
        needsLayout = true
        window?.invalidateCursorRects(for: self)
    }

    /// Reduce Motion was switched: a blinking cursor goes steady (or starts again).
    @objc func accessibilityOptionsChanged(_ note: Notification) { updateBlink() }

    public override func becomeFirstResponder() -> Bool {
        renderer.setFocused(true)
        reportFocus()
        requestRedraw()
        return true
    }

    public override func resignFirstResponder() -> Bool {
        endComposition()
        renderer.setFocused(false)
        reportFocus()
        requestRedraw()
        return true
    }

    /// The view has the keyboard while its window is key and it is the first responder.
    private func reportFocus() {
        let focused = window?.isKeyWindow == true && window?.firstResponder === self
        guard focused != focusReported else { return }
        focusReported = focused
        interaction?.focusChanged(focused)
    }
}
