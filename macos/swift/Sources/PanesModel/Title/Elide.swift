// Shortening the title row's last folder to the room the plan gives it (`view/elide.rs`): by characters, the caller
// measuring, so it is testable without fonts. A name loses its middle and keeps its extension. A view never calls this: it
// cuts with `CutText`, by the width the layout gives.

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
