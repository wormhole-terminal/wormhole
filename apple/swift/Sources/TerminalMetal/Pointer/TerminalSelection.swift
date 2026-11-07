// The selection lives in the engine; the view draws it and copies it. After each frame the spans of the rows
// just drawn come from the seam (output scrolls a selection with its text, typing clears it), and while none
// exists nothing is asked.

import AppKit
import TerminalCore

extension MetalTerminalView {
    /// Mirror the engine's selection into the renderer. Called after a frame was applied.
    func refreshSelection() {
        guard selectionMayExist, let interaction else { return }
        let spans = interaction.selectionSpans()
        renderer.setSelection(spans)
        selectionMayExist = !spans.isEmpty
        hasSelection = selectionMayExist
    }

    // MARK: Edit menu actions (the responder chain finds these)

    @objc public func copy(_ sender: Any?) {
        guard let text = interaction?.selectionText() else { return }
        let board = NSPasteboard.general
        board.clearContents()
        board.setString(text, forType: .string)
        onCopied?(text)
    }

    @objc public override func selectAll(_ sender: Any?) {
        interaction?.selectAll()
        selectionMayExist = true
        requestRedraw()
    }

    /// Erase the history and have the shell redraw its prompt (⌘K).
    @objc public func clearBuffer(_ sender: Any?) {
        interaction?.clearHistory()
        source.write(Data([0x0C]))
        requestRedraw()
    }

    public func validateUserInterfaceItem(_ item: any NSValidatedUserInterfaceItem) -> Bool {
        switch item.action {
        case #selector(copy(_:)): hasSelection
        case #selector(paste(_:)): NSPasteboard.general.canReadObject(forClasses: [NSString.self, NSURL.self], options: nil)
        default: true
        }
    }
}

extension MetalTerminalView: NSUserInterfaceValidations {}
