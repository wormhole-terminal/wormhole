// What a person does to a terminal's Probe marks, as the view needs it: opening folds, the pointer revealing a
// shortened span, finding the file a rule named. The decisions (what may be folded, when a span shows its text,
// where a path is looked for) are Rust's (`FFI-CONTRACT.md`, "Probe decorations and fold markers"); the view
// only reports gestures and draws what the frame says.

import Foundation

/// A file a hook offered or a rule's link named, looked for in the terminal's folder.
public struct ResolvedFile: Sendable, Equatable {
    public var path: String?
    /// 1-based.
    public var line: Int?
    /// The chip that says it was not found (`path` is then `nil`).
    public var words: String?

    public init(path: String? = nil, line: Int? = nil, words: String? = nil) { (self.path, self.line, self.words) = (path, line, words) }
}

public protocol TerminalProbes: AnyObject, Sendable {
    /// Whether any fold exists: Edit → Show Hidden Lines is greyed out without one.
    var hasFolds: Bool { get }
    /// The marker was clicked (or VoiceOver's Show): open or close fold `id`.
    func foldToggle(_ id: UInt64) -> Bool
    /// Fold the output of finished command block `block` (the context menu's Fold Output). Whether a fold was made.
    func foldBlockOutput(_ block: UInt64) -> Bool
    /// Open every fold; how many lines that showed (0: none was closed).
    func foldShowAll() -> Int
    /// What VoiceOver announces when `lines` lines were shown.
    func foldShownWords(lines: Int) -> String
    /// The pointer is over viewport cell (`row`, `column`) (`nil`: it left); ⌥ held is `alt`. Whether the span that shows
    /// its text changed, so the view takes a new frame.
    func setReveal(row: Int?, column: Int?, alt: Bool) -> Bool
    /// Looks at the disk: off the main thread.
    func resolvePath(_ text: String) -> ResolvedFile
    /// Opens `path` in the app `bundle` at `line` as the core says that app takes one, and starts it (`appFolder`: where Launch
    /// Services says the app is). Whether anything was started.
    func openFile(path: String, line: Int?, bundle: String, appFolder: String?) -> Bool
}
