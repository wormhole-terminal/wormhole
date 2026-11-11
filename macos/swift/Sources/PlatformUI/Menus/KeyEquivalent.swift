// A key spec as an `NSMenuItem` wants it: the character (or the private-use code of a function key) and the
// modifier mask. The menu shows the key and, for an item with an action, also binds it, so AppKit's own
// handling (⌘Q, ⌘H, the Services key) works with any first responder.

import AppKit
import PlatformModel

public struct KeyEquivalent: Equatable, Sendable {
    public var character: String
    public var modifiers: NSEvent.ModifierFlags

    private static let named: [String: Int] = [
        "up": NSUpArrowFunctionKey, "down": NSDownArrowFunctionKey, "left": NSLeftArrowFunctionKey,
        "right": NSRightArrowFunctionKey, "home": NSHomeFunctionKey, "end": NSEndFunctionKey,
        "pageup": NSPageUpFunctionKey, "pagedown": NSPageDownFunctionKey, "delete": NSDeleteFunctionKey,
    ]

    /// nil for a key a menu item can't hold (none today; Escape and Tab are not menu keys).
    public init?(_ spec: KeySpec) {
        switch spec.key {
        case "space": character = " "
        case "enter": character = "\r"
        case "backspace": character = "\u{8}"
        case "tab": character = "\t"
        case "escape": character = "\u{1b}"
        case let key where Self.named[key] != nil: character = String(UnicodeScalar(UInt32(Self.named[key]!)).map(Character.init) ?? " ")
        case let key where spec.isFunctionKey:
            character = String(Character(UnicodeScalar(UInt32(NSF1FunctionKey) + UInt32(key.dropFirst())! - 1)!))
        case let key where key.count == 1: character = key
        default: return nil
        }
        var flags: NSEvent.ModifierFlags = []
        if spec.mods.contains(.command) { flags.insert(.command) }
        if spec.mods.contains(.option) { flags.insert(.option) }
        if spec.mods.contains(.control) { flags.insert(.control) }
        // AppKit shows ⇧ for an uppercase character; a lowercase one with the mask is the same key.
        if spec.mods.contains(.shift) { flags.insert(.shift) }
        modifiers = flags
    }

    /// Applies it to an item.
    @MainActor func apply(to item: NSMenuItem) {
        item.keyEquivalent = character
        item.keyEquivalentModifierMask = modifiers
    }
}
