// Keyboard: a key goes through `interpretKeyEvents` (so an input method can compose Japanese, accents, emoji)
// unless it is a control chord, a key that types nothing, or option-as-meta: those go to the engine directly.
// AppKit then calls back into the input client (`InputClient.swift`): `insertText` for committed text,
// `setMarkedText` while composing, `doCommand` for the keys that are commands (return, arrows, backspace).
// The engine encodes the key by the modes the program set (application cursor keys, kitty keyboard,
// modifyOtherKeys), through `FrameSource.sendKey`.

import AppKit
import TerminalCore

extension MetalTerminalView {
    public override func keyDown(with event: NSEvent) {
        guard let key = KeyMap.input(from: event, optionAsMeta: optionAsMeta), !key.platform else {
            super.keyDown(with: event)
            return
        }
        if !composition.isActive, key.control || KeyMap.isNonText(key) || (key.alt && key.optionAsMeta) {
            send(key)
            return
        }
        pendingKey = key
        handledByInputMethod = false
        interpretKeyEvents([event])
        pendingKey = nil
        // Nothing consumed it (a key the input system has no opinion about): the engine takes it as it is.
        if !handledByInputMethod { send(key) }
    }

    func send(_ key: KeyInput) {
        holdCursor()
        if source.encodesKeys {
            _ = source.sendKey(key)
        } else if let bytes = KeyEncoder.encode(key) {
            source.write(bytes)
        }
        // Typing returns the view to the live screen and clears the selection on the source's side.
        requestRedraw()
    }

    /// Text that is not a key: a composed commit, a dead-key result, dictation. It goes in as typed.
    func type(_ text: String) {
        holdCursor()
        source.write(Data(text.utf8))
        requestRedraw()
    }

    // MARK: paste and drop

    @objc public func paste(_ sender: Any?) { _ = insert(from: .general) }

    /// Paste text from `board`, or the paths of files on it (escaped, each followed by a space, as Terminal.app
    /// inserts them; an agent takes a dropped image by that pasted path).
    public func insert(from board: NSPasteboard) -> Bool {
        if let urls = board.readObjects(forClasses: [NSURL.self], options: [.urlReadingFileURLsOnly: true]) as? [URL], !urls.isEmpty,
            let text = interaction?.droppedText(paths: urls.map(\.path))
        {
            source.paste(text)
        } else if let text = board.string(forType: .string), !text.isEmpty {
            source.paste(text)
        } else {
            return false
        }
        window?.makeFirstResponder(self)
        requestRedraw()
        return true
    }

    public override func draggingEntered(_ sender: any NSDraggingInfo) -> NSDragOperation { .copy }

    public override func performDragOperation(_ sender: any NSDraggingInfo) -> Bool { insert(from: sender.draggingPasteboard) }
}
