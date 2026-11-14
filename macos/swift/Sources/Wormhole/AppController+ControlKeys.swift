// The control folder's key commands (`WORMHOLE_DEVCTL`): `key <spec>` sends a key to the window as the keyboard would, through
// the application's own event dispatch (menu key equivalents, the window's monitors, then the first responder), so F6, ⇥,
// Esc and the arrows are checked on the real window without taking the focus from anything. `keys` says where they are.

import AppKit
import PlatformModel
import PlatformUI
import WindowUI

extension AppController {
    /// False when `command` is not one of the key commands.
    func runKeys(command: String, argument: String) -> Bool {
        switch command {
        case "key": reply(sendKey(argument))
        case "keys": reply("focus=\(hub.focus.detail) responder=\(keyTarget?.firstResponder.map { String(describing: type(of: $0)) } ?? "none")")
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

    /// The window the keys go to: the one `pointer-window <title>` named (Settings, the Studio), else the workspace's in front.
    private var keyTarget: NSWindow? { drivenWindow ?? window }

    /// Carbon's key codes of the ANSI letters, digits and punctuation, by position.
    private static let ansiKeys = Array("asdfhgzxcv\u{0}bqweryt123465=97-80]ou[ip\u{0}lj'k;\\,/nm.")

    /// `[shift-|cmd-|ctrl-|alt-]…key`, the key a name (`f6`, `tab`, `esc`, `left`, `return`, `space`) or one character.
    func sendKey(_ spec: String) -> String {
        var parts = spec.lowercased().split(separator: "-", omittingEmptySubsequences: false).map(String.init)
        if parts.last == "" { parts.removeLast(); parts[parts.count - 1] += "-" }
        guard let name = parts.last, let window = keyTarget else { return "error: key needs a spec like shift-f6" }
        var flags = Self.modifierFlags(parts.dropLast().joined(separator: "-"))
        if let unknown = parts.dropLast().first(where: { Self.modifierFlags($0).isEmpty }) { return "error: unknown modifier \(unknown)" }
        let named: [String: (UInt16, String)] = [
            "f6": (97, "\u{F709}"), "tab": (48, "\t"), "esc": (53, "\u{1B}"), "escape": (53, "\u{1B}"),
            "left": (123, "\u{F702}"), "right": (124, "\u{F703}"), "down": (125, "\u{F701}"), "up": (126, "\u{F700}"),
            "return": (36, "\r"), "enter": (36, "\r"), "space": (49, " "), "backspace": (51, "\u{7F}"), "delete": (51, "\u{7F}"),
        ]
        // "+" is the key of "=" on the ANSI layout, with the character "+".
        guard let (code, text) = named[name] ?? name.first.flatMap({ Self.ansiKeys.firstIndex(of: $0) }).map({ (UInt16($0), name) })
                ?? (name == "+" ? (UInt16(24), "+") : nil),
            name.count == 1 || named[name] != nil
        else { return "error: unknown key \(name)" }
        // The keyboard marks F-keys and arrows with the function flag.
        if text.unicodeScalars.first.map({ (0xF700...0xF70F).contains($0.value) }) == true { flags.insert(.function) }
        // ⌘C with nothing selected in a field is the overlay's own (the palette copies a setting's key): the view hierarchy takes
        // it before the menu does, so it goes the real way. With a selection AppKit would write the person's clipboard.
        let editor = window.firstResponder as? NSTextView
        // ⌘⌫ in a field is the field's too (the palette's ⌘⌫ closes the terminal chosen); the line editor's key is the menu's only elsewhere.
        let fieldCopy = flags == .command && editor != nil && ((name == "c" && editor?.selectedRange.length == 0) || name == "backspace")
        // The key binding ⌘⌫ means for a field (delete to the line's start) is applied directly: the input method that would
        // look it up is a really active app's.
        if fieldCopy, name == "backspace", let editor {
            editor.doCommand(by: #selector(NSResponder.deleteToBeginningOfLine(_:)))
            return "key \(spec) -> deleteToBeginningOfLine focus=\(settled())"
        }
        // In another window (`pointer-window Settings`) the key goes to that window's views first, as a person's would, a recorder's
        // among them; only what they leave goes on to the menu.
        // A key the keymap gives a view's own action (⌘← ⌘→ ⌘⌫ are the terminal's line editor: `ActionRouter.viewed`) goes the
        // keyboard's way, so the view's `performKeyEquivalent` has it before the menus, as a real key's does.
        if !fieldCopy, window === self.window, let id = menuAction(for: name, flags), !ActionRouter.viewed.contains(id) {
            // An action nothing in the table runs is AppKit's own (Edit ▸ Paste, Undo…): the menu would send it up the responder chain.
            if router.table[id] == nil, let selector = StandardActions.selectors[.action(id)] {
                // In a text field the system's own copy, cut and paste would use the person's clipboard.
                let clipboard = [#selector(NSText.copy(_:)), #selector(NSText.cut(_:)), #selector(NSText.paste(_:))].contains(selector)
                if clipboard, window.firstResponder is NSTextView { return "error: \(id.rawValue) in a text field would use the person's clipboard" }
                InFront.during(window) { _ = NSApp.sendAction(selector, to: nil, from: nil) }
            } else {
                // As a key of the active app: an action that goes up the responder chain (Copy) finds the window's first responder.
                InFront.during(window) { router.run(id) }
            }
            return "key \(spec) -> \(id.rawValue) focus=\(settled())"
        }
        press(code: code, characters: text, flags: flags, in: window)
        return "key \(spec) focus=\(settled())"
    }

    /// One character typed into whatever has the keys, as `text` sends them: its key on the ANSI layout when it has one.
    func typeCharacter(_ character: Character) {
        guard let window = keyTarget else { return }
        let code = character.lowercased().first.flatMap { Self.ansiKeys.firstIndex(of: $0) }.map(UInt16.init) ?? (character == " " ? 49 : 0)
        press(code: code, characters: String(character), flags: character.isUppercase ? .shift : [], in: window)
    }

    /// A key down and up, through the application's own dispatch (its monitors, then the window's first responder).
    private func press(code: UInt16, characters: String, flags: NSEvent.ModifierFlags, in window: NSWindow) {
        // Nothing in the workspace's window has the keys (its first responder is the window itself) and nothing covers the
        // terminal: the keys are the selected terminal's, as they are once a window has been in front. A window that never
        // really got on a screen (a hosted runner's session) never made it first responder.
        if window === self.window, window.firstResponder === window, hub.panel.cover == nil, let view = selectedView, view.window === window {
            window.makeFirstResponder(view)
        }
        for type in [NSEvent.EventType.keyDown, .keyUp] {
            NSEvent.keyEvent(
                with: type, location: .zero, modifierFlags: flags, timestamp: ProcessInfo.processInfo.systemUptime,
                windowNumber: window.windowNumber, context: nil, characters: characters, charactersIgnoringModifiers: characters,
                isARepeat: false, keyCode: code
            ).map { event in
                // A field being edited reads keys through the input method, which only a really active app has; the key
                // bindings it would apply are applied directly (typing, Return, Esc, arrows), the rest go the usual way.
                if let editor = window.firstResponder as? NSTextView, type == .keyDown, flags.isDisjoint(with: [.command, .control]) {
                    // Esc goes the window's way first: SwiftUI's `onKeyPress(.escape)` on a field (a row's rename, the find bar)
                    // hears the key there, never in the key binding. The binding (`cancelOperation:`) is applied after it only
                    // while the field still has the keys (Esc did not end it), as it was before for every field.
                    if code == 53 {
                        InFront.during(window) { NSApp.sendEvent(event) }
                        if window.firstResponder === editor { editor.interpretKeyEvents([event]) }
                        return
                    }
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
        guard mods.contains(.command) || spec.isFunctionKey || mods.isSuperset(of: [.control, .option]) else { return nil }
        return Keymap.shared.entries.first { $0.keys.contains { $0.sameKey(spec) } }?.id
    }
}
