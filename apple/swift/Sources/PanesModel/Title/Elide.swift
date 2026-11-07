// Shortening a name or a folder to fit a row (`view/elide.rs`): by characters, the caller measuring, so it
// is testable without fonts. A name loses its middle and keeps its extension; a folder loses its front.

import Foundation

public enum Elide {
    public static let ellipsis: Character = "…"

    /// `name` in at most `keep` characters (the ellipsis included), cut in the middle: the extension stays
    /// whole and the start gets the odd character.
    public static func middle(_ name: String, _ keep: Int) -> String {
        let chars = Array(name)
        if chars.count <= keep { return name }
        if keep <= 1 { return String(ellipsis) }
        // The extension, from the last dot that isn't the first character (`.gitignore` has none).
        let dot = chars.lastIndex(of: ".").flatMap { $0 > 0 ? $0 : nil }
        var (stem, ext) = dot.map { (Array(chars[..<$0]), Array(chars[$0...])) } ?? (chars, [])
        var room = keep - ext.count - 1
        // At least one character of the stem on each side of the ellipsis.
        if room < 2 { (stem, ext, room) = (chars, [], keep - 1) }
        let head = (room + 1) / 2
        return String(stem[..<head]) + String(ellipsis) + String(stem[(stem.count - (room - head))...]) + String(ext)
    }

    /// `folder` in at most `keep` characters, cut at the front: `…/adapters/session`.
    public static func front(_ folder: String, _ keep: Int) -> String {
        let chars = Array(folder)
        if chars.count <= keep { return folder }
        if keep <= 1 { return String(ellipsis) }
        let tail = Array(chars[(chars.count - (keep - 1))...])
        // The longest end that starts at a separator, so it reads `…/name`, not `…me`.
        let cut = tail.firstIndex(of: "/").map { Array(tail[$0...]) } ?? tail
        return String(ellipsis) + String(cut)
    }

    /// `text` as it fits in `max`: itself when it does, else the longest `shorten(text, keep)` that does, or
    /// the ellipsis alone. About log₂ of the length in measures.
    public static func fit(
        _ text: String, max: Double, measure: (String) -> Double, shorten: (String, Int) -> String
    ) -> String {
        if measure(text) <= max { return text }
        var (low, high) = (1, text.count - 1)
        var best = String(ellipsis)
        while low <= high {
            let keep = low + (high - low) / 2
            let candidate = shorten(text, keep)
            if measure(candidate) <= max {
                best = candidate
                low = keep + 1
            } else {
                high = keep - 1
            }
        }
        return best
    }
}
