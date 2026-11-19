// The Command shortcuts that belong to the terminal, by the app's keymap (`keymap::terminal_action`, asked through
// `terminalAction`): copy, paste, select all, clear, the three that edit the shell's line (as Terminal.app and iTerm2
// do) and the jumps between prompts. The rest of the Command keys are the application's menus.

import AppKit
import TerminalCore

enum TerminalShortcut: Equatable {
    case copy, paste, selectAll, clear
    /// Bytes for the shell's line editor: ⌘⌫ deletes the line (^U), ⌘← and ⌘→ jump (^A, ^E).
    case send([UInt8])
    /// ⌘↑ / ⌘↓: the previous or next prompt; taken even without shell integration, where it finds none.
    case prompt(PromptDirection)

    /// What the keymap's action for the key does here (`platform_command`): the table says which keys, this the effect.
    init?(action: String?) {
        switch action {
        case "copy": self = .copy
        case "paste": self = .paste
        case "selectAll": self = .selectAll
        case "clearBuffer": self = .clear
        case "deleteLine": self = .send([0x15])
        case "lineStart": self = .send([0x01])
        case "lineEnd": self = .send([0x05])
        case "previousPrompt": self = .prompt(.up)
        case "nextPrompt": self = .prompt(.down)
        default: return nil
        }
    }
}

extension MetalTerminalView {
    /// What a focused terminal does with a ⌘ key: the table's, else the same key with ⌘ alone (⇧⌘V and ⌥⌘C as ⌘V and ⌘C,
    /// ⇧⌘↑ as ⌘↑) unless some row of the table uses the key (`platform_command`, `input.rs:105`).
    private func shortcut(for event: NSEvent) -> TerminalShortcut? {
        // The four modifiers only: caps lock or the function flag an arrow carries is not a different key.
        let flags = event.modifierFlags.intersection([.command, .shift, .option, .control])
        guard flags.contains(.command), let name = KeyMap.input(from: event, optionAsMeta: optionAsMeta)?.key else { return nil }
        if let exact = TerminalShortcut(action: terminalAction(name, flags)) { return exact }
        guard flags != .command, !isAppKey(name, flags) else { return nil }
        return TerminalShortcut(action: terminalAction(name, .command))
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
        case .prompt(let direction): onPrompt(direction)
        }
        return true
    }
}
