// While a shortcut recorder has the keys, every press is its to record, ⌘ combinations included: GPUI's recorder sees every
// key first (`settings_window/window.rs` `record_key`). AppKit hands a key to the app's local monitors before any view, so
// the one rule is here and each monitor asks it first: a press for a recorder is left alone.

import AppKit

/// A view that records the next press as a shortcut while it is its window's first responder (and only then).
@MainActor
public protocol RecordsKeys: NSView {}

extension NSEvent {
    /// Whether a shortcut recorder has the keys of the window this press goes to: no monitor of the app takes it then.
    @MainActor public var goesToRecorder: Bool { window?.firstResponder is any RecordsKeys }
}
