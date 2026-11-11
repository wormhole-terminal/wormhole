// The panel window: non-activating, so it can take the keyboard while another app stays active and Wormhole never
// comes forward over it. Borderless and transparent: the card is the SwiftUI view. ⌘⏎ never reaches a field as an
// editing command, so the window turns it into the panel's own key.

import AppKit

final class QuickPanelWindow: NSPanel {
    var onCommandReturn: () -> Void = {}

    override var canBecomeKey: Bool { true }
    override var canBecomeMain: Bool { false }

    override func sendEvent(_ event: NSEvent) {
        if event.type == .keyDown, event.modifierFlags.intersection(.deviceIndependentFlagsMask) == .command, event.keyCode == 36 {
            return onCommandReturn()
        }
        super.sendEvent(event)
    }
}
