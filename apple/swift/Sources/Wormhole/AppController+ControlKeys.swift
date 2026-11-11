// The control folder's key commands (`WORMHOLE_DEVCTL`): `key <spec>` sends a key to the window as the keyboard would, through
// the application's own event dispatch (menu key equivalents, the window's monitors, then the first responder), so F6, ⇥,
// Esc and the arrows are checked on the real window without taking the focus from anything. `keys` says where they are.

import AppKit
import PlatformModel
import WindowUI

extension AppController {
    /// False when `command` is not one of the key commands.
    func runKeys(command: String, argument: String) -> Bool {
        switch command {
        case "key": reply(sendKey(argument))
        case "keys": reply("focus=\(hub.focus.detail)")
        case "hold-cmd", "release-cmd":
            // ⌘ pressed alone and kept down (the ⌘1-9 hints show after 250 ms), or let go: the modifier event the window's monitors see.
            let flags: NSEvent.ModifierFlags = command == "hold-cmd" ? .command : []
            if let window, let event = NSEvent.keyEvent(
                with: .flagsChanged, location: .zero, modifierFlags: flags, timestamp: ProcessInfo.processInfo.systemUptime,
                windowNumber: window.windowNumber, context: nil, characters: "", charactersIgnoringModifiers: "", isARepeat: false, keyCode: 55)
            {
                NSApp.sendEvent(event)
            }
            reply(command)
        default: return false
        }
        return true
    }

    /// Carbon's key codes of the ANSI letters, digits and punctuation, by position.
    private static let ansiKeys = Array("asdfhgzxcv\u{0}bqweryt123465=97-80]ou[ip\u{0}lj'k;\\,/nm.")

    /// `[shift-|cmd-|ctrl-|alt-]…key`, the key a name (`f6`, `tab`, `esc`, `left`, `return`, `space`) or one character.
    private func sendKey(_ spec: String) -> String {
        var parts = spec.lowercased().split(separator: "-", omittingEmptySubsequences: false).map(String.init)
        if parts.last == "" { parts.removeLast(); parts[parts.count - 1] += "-" }
        guard let name = parts.last, let window else { return "error: key needs a spec like shift-f6" }
        var flags = Self.modifierFlags(parts.dropLast().joined(separator: "-"))
        if let unknown = parts.dropLast().first(where: { Self.modifierFlags($0).isEmpty }) { return "error: unknown modifier \(unknown)" }
        let named: [String: (UInt16, String)] = [
            "f6": (97, "\u{F709}"), "tab": (48, "\t"), "esc": (53, "\u{1B}"), "escape": (53, "\u{1B}"),
            "left": (123, "\u{F702}"), "right": (124, "\u{F703}"), "down": (125, "\u{F701}"), "up": (126, "\u{F700}"),
            "return": (36, "\r"), "enter": (36, "\r"), "space": (49, " "),
        ]
        guard let (code, text) = named[name] ?? name.first.flatMap({ Self.ansiKeys.firstIndex(of: $0) }).map({ (UInt16($0), name) }),
            name.count == 1 || named[name] != nil
        else { return "error: unknown key \(name)" }
        // The keyboard marks F-keys and arrows with the function flag.
        if text.unicodeScalars.first.map({ (0xF700...0xF70F).contains($0.value) }) == true { flags.insert(.function) }
        if let id = menuAction(for: name, flags) {
            router.run(id)
            return "key \(spec) -> \(id.rawValue) focus=\(settled())"
        }
        press(code: code, characters: text, flags: flags, in: window)
        return "key \(spec) focus=\(settled())"
    }

    /// One character typed into whatever has the keys, as `text` sends them: its key on the ANSI layout when it has one.
    func typeCharacter(_ character: Character) {
        guard let window else { return }
        let code = character.lowercased().first.flatMap { Self.ansiKeys.firstIndex(of: $0) }.map(UInt16.init) ?? (character == " " ? 49 : 0)
        press(code: code, characters: String(character), flags: character.isUppercase ? .shift : [], in: window)
    }

    /// A key down and up, through the application's own dispatch (its monitors, then the window's first responder).
    private func press(code: UInt16, characters: String, flags: NSEvent.ModifierFlags, in window: NSWindow) {
        for type in [NSEvent.EventType.keyDown, .keyUp] {
            NSEvent.keyEvent(
                with: type, location: .zero, modifierFlags: flags, timestamp: ProcessInfo.processInfo.systemUptime,
                windowNumber: window.windowNumber, context: nil, characters: characters, charactersIgnoringModifiers: characters,
                isARepeat: false, keyCode: code
            ).map { event in
                // A field being edited reads keys through the input method, which only a really active app has; the key
                // bindings it would apply are applied directly (typing, Return, Esc, arrows), the rest go the usual way.
                if let editor = window.firstResponder as? NSTextView, type == .keyDown, flags.isDisjoint(with: [.command, .control]) {
                    return editor.interpretKeyEvents([event])
                }
                InFront.during(window) { NSApp.sendEvent(event) }
            }
        }
    }

    /// Where the keys are once SwiftUI has acted on a request for focus, which it does on the next turn of the run loop.
    private func settled() -> String {
        RunLoop.current.run(until: Date(timeIntervalSinceNow: 0.1))
        return hub.focus.detail
    }

    /// A key equivalent (F6, the ⌘-keys) is the menu bar's before it is the window's. A background instance has no menu bar
    /// to ask, so the keymap that builds the menus says which action the key is, and the same router runs it.
    private func menuAction(for name: String, _ flags: NSEvent.ModifierFlags) -> ActionID? {
        var mods: KeyMods = []
        if flags.contains(.shift) { mods.insert(.shift) }
        if flags.contains(.command) { mods.insert(.command) }
        if flags.contains(.control) { mods.insert(.control) }
        if flags.contains(.option) { mods.insert(.option) }
        let spec = KeySpec(mods, name == "esc" ? "escape" : name)
        guard mods.contains(.command) || spec.isFunctionKey else { return nil }
        return Keymap.shared.entries.first { $0.keys.contains { $0.sameKey(spec) } }?.id
    }
}
