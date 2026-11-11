// What a test instance (`--background`) must not do to the person's screen and ears (GPUI's `background.rs`): its
// windows open behind the others and it never makes a sound. One value, set at launch, that every window and the
// bell ask, so no feature has to remember.

import AppKit

@MainActor
public enum TestInstance {
    /// Set once at launch from the options.
    public static var isBackground = false

    /// Brings `window` forward and makes it the key window; in a test instance it goes behind the others instead.
    public static func bring(_ window: NSWindow?) {
        if isBackground { window?.orderBack(nil) } else { window?.makeKeyAndOrderFront(nil) }
    }

    /// Whether the app may make a sound (the bell).
    public static var mayMakeSound: Bool { !isBackground }
}
