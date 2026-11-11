// `TerminalInteraction` over the Rust terminal: each call is one seam call, with the types translated. The
// generated bindings and TerminalCore name some types alike (`RowSpan`, `PointerButton` ...), so both are
// written with their module here.

import Foundation
import TerminalCore
import WormholeBindings

final class UniFFIInteraction: TerminalCore.TerminalInteraction {
    private let terminal: WormholeBindings.Terminal
    init(_ terminal: WormholeBindings.Terminal) { self.terminal = terminal }

    var takesPointer: Bool { terminal.takesPointer() }

    func reportPointer(_ input: PointerInput) -> Bool {
        terminal.reportPointer(
            event: WormholeBindings.PointerEvent(
                button: Self.button(input.button), action: Self.action(input.action), col: UInt32(input.column),
                row: UInt32(input.row), x: UInt32(input.x), y: UInt32(input.y), shift: input.modifiers.shift,
                alt: input.modifiers.alt, control: input.modifiers.control))
    }

    func scrollWheel(_ input: WheelInput) {
        terminal.scrollWheel(
            step: WheelStep(
                lines: input.lines, columns: input.columns, col: UInt32(input.column), row: UInt32(input.row),
                x: UInt32(input.x), y: UInt32(input.y), shift: input.modifiers.shift, alt: input.modifiers.alt,
                control: input.modifiers.control))
    }

    func focusChanged(_ focused: Bool) { terminal.focusChanged(focused: focused) }
    func droppedText(paths: [String]) -> String { terminal.droppedText(paths: paths) }
    func clearHistory() -> Bool { terminal.clearHistory() }
    func pasteClipboard(_ text: String?) { terminal.pasteClipboard(clipboard: text) }
    func programCopied(_ text: String, replacing before: String?) { terminal.programCopied(text: text, clipboardBefore: before) }

    func selectionStart(kind: TerminalCore.SelectionKind, row: Int, column: Int, side: TerminalCore.CellSide) {
        let kind: WormholeBindings.SelectionKind =
            switch kind {
            case .cell: .cell
            case .word: .word
            case .line: .line
            case .block: .block
            }
        terminal.selectionStart(kind: kind, row: UInt32(row), col: UInt32(column), side: Self.side(side))
    }

    func selectionUpdate(row: Int, column: Int, side: TerminalCore.CellSide) {
        terminal.selectionUpdate(row: UInt32(row), col: UInt32(column), side: Self.side(side))
    }

    func selectionFinish() { terminal.selectionFinish() }
    func selectAll() { terminal.selectAll() }
    func selectionText() -> String? { terminal.selectionText() }
    func selectionSpans() -> [TerminalCore.RowSpan] { terminal.selectionSpans().map(Self.span) }

    func link(row: Int, column: Int) -> LinkInfo? {
        terminal.linkAt(row: UInt32(row), col: UInt32(column)).map {
            LinkInfo(uri: $0.uri, isExplicit: $0.explicit, spans: $0.spans.map(Self.span), isFile: $0.file, readout: $0.readout)
        }
    }

    func openTarget(for uri: String) -> TerminalCore.OpenTarget {
        switch terminal.openTarget(uri: uri) {
        case .url(let url): .url(url)
        case .reveal(let path): .reveal(path: path)
        case .refused: .refused
        }
    }

    private static func span(_ span: WormholeBindings.RowSpan) -> TerminalCore.RowSpan {
        TerminalCore.RowSpan(row: Int(span.row), first: Int(span.first), last: Int(span.last))
    }

    private static func button(_ button: TerminalCore.PointerButton) -> WormholeBindings.PointerButton {
        switch button {
        case .left: .left
        case .middle: .middle
        case .right: .right
        case .none: .none
        }
    }

    private static func action(_ action: TerminalCore.PointerAction) -> WormholeBindings.PointerAction {
        switch action {
        case .press: .press
        case .release: .release
        case .motion: .motion
        }
    }

    private static func side(_ side: TerminalCore.CellSide) -> WormholeBindings.CellSide { side == .left ? .left : .right }
}
