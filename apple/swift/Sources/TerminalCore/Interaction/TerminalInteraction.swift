// What a person does to a terminal besides typing a key, as the view needs it: the pointer, the wheel,
// selecting, links, paste and focus. The real implementation is the Rust seam (`apple/FFI-CONTRACT.md`,
// "Interaction"), which holds the selection and decides who gets the pointer; the view only translates
// `NSEvent`s into these calls. Positions are viewport cells, row 0 at the top of what the last frame showed.

import Foundation

public enum PointerButton: Sendable, Equatable { case left, middle, right, none }
public enum PointerAction: Sendable, Equatable { case press, release, motion }

public struct PointerModifiers: Sendable, Equatable {
    public var shift = false
    public var alt = false
    public var control = false
    public init(shift: Bool = false, alt: Bool = false, control: Bool = false) {
        self.shift = shift
        self.alt = alt
        self.control = control
    }
}

/// A button or motion over the grid; `x`, `y` are points from the grid's corner (for programs that ask for pixels).
public struct PointerInput: Sendable, Equatable {
    public var button: PointerButton
    public var action: PointerAction
    public var row: Int
    public var column: Int
    public var x: Int
    public var y: Int
    public var modifiers: PointerModifiers
    public init(button: PointerButton, action: PointerAction, row: Int, column: Int, x: Int = 0, y: Int = 0, modifiers: PointerModifiers = .init()) {
        self.button = button
        self.action = action
        self.row = row
        self.column = column
        self.x = x
        self.y = y
        self.modifiers = modifiers
    }
}

/// Wheel or trackpad movement in cells (positive: up, left); fractions are kept by the receiver.
public struct WheelInput: Sendable, Equatable {
    public var lines: Float
    public var columns: Float
    public var row: Int
    public var column: Int
    public var x: Int
    public var y: Int
    public var modifiers: PointerModifiers
    public init(lines: Float, columns: Float, row: Int, column: Int, x: Int = 0, y: Int = 0, modifiers: PointerModifiers = .init()) {
        self.lines = lines
        self.columns = columns
        self.row = row
        self.column = column
        self.x = x
        self.y = y
        self.modifiers = modifiers
    }
}

/// What a drag extends by.
public enum SelectionKind: Sendable, Equatable { case cell, word, line, block }
/// Which half of a cell the pointer is over.
public enum CellSide: Sendable, Equatable { case left, right }

/// The columns `first...last` of one viewport row.
public struct RowSpan: Sendable, Equatable {
    public var row: Int
    public var first: Int
    public var last: Int
    public init(row: Int, first: Int, last: Int) {
        self.row = row
        self.first = first
        self.last = last
    }
}

public struct LinkInfo: Sendable, Equatable {
    public var uri: String
    /// A program's own hyperlink (OSC 8) rather than a URL found in the text.
    public var isExplicit: Bool
    public var spans: [RowSpan]
    public init(uri: String, isExplicit: Bool, spans: [RowSpan]) {
        self.uri = uri
        self.isExplicit = isExplicit
        self.spans = spans
    }
}

/// What opening a link means; only the kinds the app allows.
public enum OpenTarget: Sendable, Equatable {
    case url(String)
    /// A `file:` link on this machine: shown in the Finder, never run.
    case reveal(path: String)
    case refused
}

public protocol TerminalInteraction: AnyObject, Sendable {
    /// Give the pointer to the program if it asked for it (and Shift is not held); `false`: the view handles it.
    func reportPointer(_ input: PointerInput) -> Bool
    /// A program asked for the mouse, so a click is not a selection.
    var takesPointer: Bool { get }
    func scrollWheel(_ input: WheelInput)
    func focusChanged(_ focused: Bool)
    /// The shell-escaped text for dropped files.
    func droppedText(paths: [String]) -> String
    func clearHistory()

    func selectionStart(kind: SelectionKind, row: Int, column: Int, side: CellSide)
    func selectionUpdate(row: Int, column: Int, side: CellSide)
    func selectionFinish()
    func selectAll()
    func selectionText() -> String?
    /// The selection on the rows of the last frame.
    func selectionSpans() -> [RowSpan]

    func link(row: Int, column: Int) -> LinkInfo?
    func openTarget(for uri: String) -> OpenTarget
}
