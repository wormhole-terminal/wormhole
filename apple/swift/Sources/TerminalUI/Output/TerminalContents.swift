// The terminal's text for Print and Export (`wormhole_term::contents`: the screen, or the whole history with it, as
// plain text with soft-wrapped rows joined). The seam the FFI will back.

import Foundation

public enum ContentsScope: Sendable { case screen, everything, selection }

@MainActor
public protocol TerminalContents: AnyObject {
    /// The text of `scope`; nil where there is none (no selection).
    func text(_ scope: ContentsScope) -> String?
    /// The terminal's title, for the file's and the print job's name.
    var title: String { get }
}
