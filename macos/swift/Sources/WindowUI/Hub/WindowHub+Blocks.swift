// The Edit menu's verbs on command blocks, and the View menu's minimap: ⌘↑ / ⌘↓, Select Output, Copy Output, Copy
// Command, Put Command at Prompt. The blocks are the selected terminal's (`TerminalOverlay.blocks`, Rust's engine
// behind it); the pane's context menu names one by its id.

import PlatformModel
import TerminalUI
import WorkspaceModel

extension WindowHub {
    /// ⌘↑ / ⌘↓: the previous or next prompt comes to the top of the view.
    public func jumpPrompt(_ direction: PromptDirection) {
        guard let overlay = selectedOverlay else { return }
        overlay.blocks.jumpPrompt(direction)
        overlay.redraw()
    }

    /// An action on the block `id` (the context menu's), or on the target block: the one the last jump landed on,
    /// else the newest whose command ran (the Edit menu's).
    public func blockAction(_ action: BlockController.Action, on id: UInt64? = nil) {
        guard let overlay = selectedOverlay else { return }
        let blocks = overlay.blocks
        blocks.perform(action, on: id.flatMap(blocks.block(id:)))
        overlay.redraw()
    }

    /// The context menu's Fold Output on block `id`.
    public func foldOutput(block id: UInt64?) {
        guard let id else { return }
        selectedOverlay?.foldOutput(block: id)
    }

    /// What the context menu knows of the block under the pointer in terminal `id`: none where no command ran.
    public func contextBlock(in id: TerminalID) -> TerminalMenuFacts.Block? {
        guard let overlay = overlay(for: id), let row = terminals.existing(id)?.pointerRow,
            let block = overlay.blocks.block(underRow: row)
        else { return nil }
        return TerminalMenuFacts.Block(
            id: block.id, hasCommand: !block.command.isEmpty, hasOutput: block.outputLines != nil,
            atPrompt: overlay.blocks.isAtPrompt, canFold: foldsOn && block.outputLines != nil)
    }
}

extension WindowHub {
    /// The block the Edit menu's actions act on, as one line (the control folder's `block`), and how far the view is scrolled back.
    public var blockTargetSummary: String {
        guard let overlay = selectedOverlay else { return "no terminal" } // text: control state line
        let offset = overlay.blocks.displayOffset
        guard let block = overlay.blocks.target else { return "no target offset=\(offset)" } // text: control state line
        return "target lines=\(block.start)...\(block.end) output=\(block.outputLines.map { "\($0)" } ?? "none") command=\(block.command) offset=\(offset)" // text: control state line
    }
}
