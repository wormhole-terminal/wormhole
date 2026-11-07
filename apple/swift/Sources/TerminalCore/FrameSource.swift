// The seam between the Metal view and whatever produces a screen: the synthetic `MockFrameSource`, or
// `UniFFIFrameSource` over the Rust terminal. The view knows only this protocol, so the renderer is
// developed and measured without Rust and bound to the real engine by swapping one object.

import Foundation

/// The grid's size: cells for the program, points per cell for programs that ask (`CSI 16 t`).
public struct GridSize: Sendable, Equatable {
    public var cols: Int
    public var rows: Int
    public var cellWidth: Float
    public var cellHeight: Float

    public init(cols: Int, rows: Int, cellWidth: Float = 8, cellHeight: Float = 16) {
        self.cols = cols
        self.rows = rows
        self.cellWidth = cellWidth
        self.cellHeight = cellHeight
    }
}

/// What a terminal's program did, as the view needs to hear it (the contract's `TermEvent`).
public enum TerminalEvent: Sendable, Equatable {
    /// New content: take a frame.
    case output
    case title(String)
    case resetTitle
    case bell
    case exited(code: Int32?)
    /// A program set the clipboard and Settings allow it: put `text` on the pasteboard.
    case clipboardWrite(String)
    /// A program may read the clipboard: `giveClipboard` its text.
    case clipboardRead
    /// A program asks to read the clipboard and the person decides: `answerClipboard`.
    case clipboardAsk
    /// The question went away unanswered (it timed out).
    case clipboardAskClosed
}

/// A key press in the app's key vocabulary ("enter", "left", "f5", "a"), as `Terminal::send_key` takes it.
public struct KeyInput: Sendable, Equatable {
    public var key: String
    /// What the key types with the layout and modifiers; `nil` for keys that type nothing.
    public var text: String?
    public var shift: Bool
    public var alt: Bool
    public var control: Bool
    public var platform: Bool
    public var optionAsMeta: Bool

    public init(
        key: String, text: String? = nil, shift: Bool = false, alt: Bool = false, control: Bool = false,
        platform: Bool = false, optionAsMeta: Bool = true
    ) {
        self.key = key
        self.text = text
        self.shift = shift
        self.alt = alt
        self.control = control
        self.platform = platform
        self.optionAsMeta = optionAsMeta
    }
}

public enum FrameSourceError: Error, Equatable {
    case notIsolated(String)
    case startFailed(String)
}

/// A screen the view draws and a program it types into. Everything but `start`'s wake callback is
/// called from the main thread; the wake callback may be called from any thread, with no lock held,
/// at most once between two `drainEvents` calls, and must only note the wake and hop to the main queue.
public protocol FrameSource: AnyObject, Sendable {
    /// Begin producing output for a grid of `size`; `wake` says there is something to drain.
    func start(size: GridSize, wake: @escaping @Sendable () -> Void) throws
    /// End the program and stop producing. Explicit: a `deinit` only frees.
    func stop()

    /// The rows that changed since the last call, packed (`PackedFrame.swift`): every row the first
    /// time and after a resize. Empty after `stop`.
    func frame() -> Data
    /// Every row, whatever changed: for a renderer that lost its picture.
    func fullFrame() -> Data
    /// Everything queued since the last drain; re-arms the wake. Call it before `frame()`: output that
    /// arrives between the two is in the frame and wakes again.
    func drainEvents() -> [TerminalEvent]

    /// Bytes for the program (typed text, a paste already encoded).
    func write(_ bytes: Data)
    /// Whether the source encodes key presses itself (the real engine knows the keyboard modes the program
    /// set). When it does not, the view encodes the key and `write`s the bytes.
    var encodesKeys: Bool { get }
    /// A key press for the source to encode; whether it sent anything. Only called when `encodesKeys`.
    func sendKey(_ key: KeyInput) -> Bool
    /// Pasted text; the source adds bracketed-paste markers when the program asked for them.
    func paste(_ text: String)

    func resize(_ size: GridSize)
    /// Scroll the view by `lines` (positive: back into history).
    func scroll(lines: Int)

    /// The terminal's text for Print and Export Text As: the screen, or with `history` everything the terminal keeps
    /// (soft-wrapped rows joined); `nil` when the source has none.
    func plainText(history: Bool) -> String?

    /// The picture a placement's key names (`Images/`): `nil` while it is still being decoded, or when the source has
    /// let it go. Asked once per picture; the view keeps the texture.
    func inlineImage(key: UInt32) -> InlineImage?

    /// The pointer, selection and links, when the source has them (the real engine and the mock do).
    var interaction: (any TerminalInteraction)? { get }
    /// Find, command blocks and the minimap over the whole history, when the source has one (the real engine).
    var scrollback: (any TerminalScrollback)? { get }

    /// What programs may do with the clipboard (OSC 52): Settings' `clipboard_read` and `clipboard_write`.
    func setClipboardRules(read: ClipboardReadRule, writeAllowed: Bool)
    /// The terminal is shown where the person can see it: the only state in which a read is asked about.
    func setOnScreen(_ onScreen: Bool)
    /// The text a read that was let through gets.
    func giveClipboard(_ text: String)
    /// The person's answer to the question; `text` is the pasteboard's, read only for a yes.
    func answerClipboard(_ choice: ClipboardChoice, text: String)
    /// The program in front as the sidebar names it, who a question names.
    func foregroundProgram() -> String?
}

extension FrameSource {
    public var encodesKeys: Bool { false }
    public func sendKey(_ key: KeyInput) -> Bool { false }
    public func paste(_ text: String) {
        write(Data(text.replacingOccurrences(of: "\r\n", with: "\r").replacingOccurrences(of: "\n", with: "\r").utf8))
    }
    public func scroll(lines: Int) {}
    public func inlineImage(key: UInt32) -> InlineImage? { nil }
    public func plainText(history: Bool) -> String? { nil }
    public var interaction: (any TerminalInteraction)? { nil }
    public var scrollback: (any TerminalScrollback)? { nil }
    public func setClipboardRules(read: ClipboardReadRule, writeAllowed: Bool) {}
    public func setOnScreen(_ onScreen: Bool) {}
    public func giveClipboard(_ text: String) {}
    public func answerClipboard(_ choice: ClipboardChoice, text: String) {}
    public func foregroundProgram() -> String? { nil }
}

/// `clipboard_read` in Settings.
public enum ClipboardReadRule: String, Sendable { case ask, allow, deny }

/// What the person chose on a program's question about the clipboard.
public enum ClipboardChoice: Sendable { case once, always, deny }
