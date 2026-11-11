// A program's use of the clipboard (OSC 52, `terminal_view/pane_bar.rs`): what is allowed is the core's (the two settings,
// the gate that asks once at a time and only where the person can see it); this is the pasteboard side of it and the
// hooks the pane's bar hangs on. A read is answered with the text or with nothing, whatever the person does.

import AppKit
import DesignSystem
import TerminalCore

extension MetalTerminalView {
    /// Settings' `clipboard_read` and `clipboard_write` for this terminal's programs.
    public func setClipboardRules(read: ClipboardReadRule, writeAllowed: Bool) {
        source.setClipboardRules(read: read, writeAllowed: writeAllowed)
    }

    /// The person's choice on the question the bar asked.
    public func answerClipboard(_ choice: ClipboardChoice) {
        source.answerClipboard(choice, text: choice == .deny ? "" : Self.pasteboardText)
    }

    /// Shown in the window in front, where the person can see a question: the core asks about a read only then (GPUI's
    /// `in_front`: the terminal is on screen and its window is the key one).
    func reportOnScreen() { source.setOnScreen(window?.isKeyWindow ?? false) }

    func handleClipboard(_ event: TerminalEvent) {
        switch event {
        case .clipboardWrite(let text):
            // What the clipboard held goes to the engine first: ⌘V over the selection this copy came from pastes it.
            interaction?.programCopied(text, replacing: Self.pasteboardText.isEmpty ? nil : Self.pasteboardText)
            Pasteboard.setText(text)
            onProgramCopy?(text)
        case .clipboardRead: source.giveClipboard(Self.pasteboardText)
        case .clipboardAsk: onClipboardAsk?(source.foregroundProgram())
        case .clipboardAskClosed: onClipboardAskClosed?()
        default: break
        }
    }

    private static var pasteboardText: String { Pasteboard.text() ?? "" }
}
