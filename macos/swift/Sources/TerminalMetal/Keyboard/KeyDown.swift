// Keyboard: a key goes through `interpretKeyEvents` (so an input method can compose Japanese, accents, emoji)
// unless it is a control chord, a key that types nothing, or option-as-meta: those go to the engine directly.
// AppKit then calls back into the input client (`InputClient.swift`): `insertText` for committed text,
// `setMarkedText` while composing, `doCommand` for the keys that are commands (return, arrows, backspace).
// The engine encodes the key by the modes the program set (application cursor keys, kitty keyboard,
// modifyOtherKeys), through `FrameSource.sendKey`.

import AppKit
import DesignSystem
import TerminalCore

extension MetalTerminalView {
    public override func keyDown(with event: NSEvent) {
        // An F-key that is the window's (a project action's) never reaches the program; a ⌘ key was asked about as a
        // key equivalent already.
        if !event.modifierFlags.contains(.command), windowClaims(event) { return }
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
        onInput?()
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
        onInput?()
        holdCursor()
        source.write(Data(text.utf8))
        requestRedraw()
    }

    // MARK: paste and drop

    @objc public func paste(_ sender: Any?) { _ = insert(from: .general, replacingProgramSelection: true) }

    /// Paste text from `board`, or the paths of files on it (escaped, each followed by a space, as Terminal.app
    /// inserts them; an agent takes a dropped image by that pasted path).
    /// `replacingProgramSelection` (⌘V, not a drop): over a selection a full-screen program drew itself the paste replaces it.
    public func insert(from board: NSPasteboard, replacingProgramSelection: Bool = false) -> Bool {
        onInput?()
        // The clipboard is `Pasteboard`'s (a test instance's is a file); any other board is a drop's, or a script's own.
        let general = board === NSPasteboard.general
        if !(general && Pasteboard.isDiverted), let urls = board.readObjects(forClasses: [NSURL.self], options: [.urlReadingFileURLsOnly: true]) as? [URL], !urls.isEmpty,
            let text = interaction?.droppedText(paths: urls.map(\.path))
        {
            source.paste(text)
        } else if let text = general ? Pasteboard.text() : board.string(forType: .string), !text.isEmpty {
            // Only the real engine (`encodesKeys`) knows a program's own selection.
            if replacingProgramSelection, source.encodesKeys, let interaction { interaction.pasteClipboard(text) } else { source.paste(text) }
        } else {
            return false
        }
        window?.makeFirstResponder(self)
        requestRedraw()
        return true
    }

    public override func draggingEntered(_ sender: any NSDraggingInfo) -> NSDragOperation {
        showDropTint(true)
        return .copy
    }

    public override func draggingExited(_ sender: (any NSDraggingInfo)?) { showDropTint(false) }
    public override func draggingEnded(_ sender: any NSDraggingInfo) { showDropTint(false) }

    public override func performDragOperation(_ sender: any NSDraggingInfo) -> Bool {
        showDropTint(false)
        return insert(from: sender.draggingPasteboard)
    }

    /// A tint, not a border, so the grid keeps its size while files hover over it (`render/mod.rs:292`).
    func showDropTint(_ on: Bool) {
        guard on else { return dropLayer?.removeFromSuperlayer() ?? () }
        let layer = dropLayer ?? CALayer()
        layer.frame = bounds
        layer.zPosition = 9
        layer.backgroundColor = dropTint.cgColor
        if layer.superlayer == nil { self.layer?.addSublayer(layer) }
        dropLayer = layer
    }
}
