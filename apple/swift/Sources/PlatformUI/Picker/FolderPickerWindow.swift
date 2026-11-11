// The picker's panel window: borderless, non-activating (it takes the keys without activating the app, so it never
// comes over other apps) and a child of the app window, which it follows and goes with.

import AppKit

final class FolderPickerWindow: NSPanel {
    override var canBecomeKey: Bool { true }
    override var canBecomeMain: Bool { false }
}
