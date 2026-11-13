// The Command shortcuts that belong to the terminal, by the app's keymap (`keymap::terminal_action`): copy,
// paste, select all, clear, and the three that edit the shell's line (as Terminal.app and iTerm2 do). The
// rest of the Command keys are the application's menus.

import AppKit

enum TerminalShortcut: Equatable {
    case copy, paste, selectAll, clear
    /// Bytes for the shell's line editor: ⌘⌫ deletes the line (^U), ⌘← and ⌘→ jump (^A, ^E).
    case send([UInt8])

    /// `keyCode` is the US-layout virtual key code, `characters` the key with no modifiers.
    static func of(characters: String?, keyCode: UInt16, flags: NSEvent.ModifierFlags) -> TerminalShortcut? {
        let flags = flags.intersection(.deviceIndependentFlagsMask)
        guard flags.contains(.command), flags.isDisjoint(with: [.control, .option]) else { return nil }
        // ⇧⌘C copies too (the app has always done so); every other shortcut is ⌘ alone, since ⇧⌘A and the
        // like are the application's.
        if characters?.lowercased() == "c", flags.subtracting(.shift) == .command { return .copy }
        guard flags == .command else { return nil }
        switch characters?.lowercased() {
        case "v": return .paste
        case "a": return .selectAll
        case "k": return .clear
        default: break
        }
        switch keyCode {
        case 51: return .send([0x15])
        case 123: return .send([0x01])
        case 124: return .send([0x05])
        default: return nil
        }
    }
}

extension MetalTerminalView {
    /// What a focused terminal does with a ⌘ key: the table's, else the same key with ⌘ alone (⇧⌘V pastes) unless it is an app key
    /// (`input.rs:105`).
    private func shortcut(for event: NSEvent) -> TerminalShortcut? {
        let characters = event.charactersIgnoringModifiers, flags = event.modifierFlags.intersection(.deviceIndependentFlagsMask)
        if let exact = TerminalShortcut.of(characters: characters, keyCode: event.keyCode, flags: flags) { return exact }
        guard flags.contains(.command), flags != .command, !isAppKey(characters ?? "", flags) else { return nil }
        return TerminalShortcut.of(characters: characters, keyCode: event.keyCode, flags: .command)
    }

    /// Whether the window took the key before the terminal (`claim_window_key`): only a ⌘ key or an F-key is asked
    /// about, so typing never gets this far.
    func windowClaims(_ event: NSEvent) -> Bool {
        guard event.type == .keyDown, let key = KeyMap.input(from: event, optionAsMeta: optionAsMeta) else { return false }
        let function = key.key.hasPrefix("f") && Int(key.key.dropFirst()) != nil
        guard key.platform || function else { return false }
        return claimKey(key.key, event.modifierFlags.intersection(.deviceIndependentFlagsMask))
    }

    public override func performKeyEquivalent(with event: NSEvent) -> Bool {
        guard window?.firstResponder === self else { return super.performKeyEquivalent(with: event) }
        // The window's key first: a project action's key wins over the terminal's own ⌘ keys and over the menus.
        if windowClaims(event) { return true }
        guard let shortcut = shortcut(for: event) else { return super.performKeyEquivalent(with: event) }
        switch shortcut {
        case .copy: copy(nil)
        case .paste: paste(nil)
        case .selectAll: selectAll(nil)
        case .clear: clearBuffer(nil)
        case .send(let bytes): source.write(Data(bytes))
        }
        return true
    }
}
