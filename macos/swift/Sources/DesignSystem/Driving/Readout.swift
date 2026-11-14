// text-file: the control folder's reply format (dev tooling), not interface text
// The one shape the control folder's read-outs answer in (`settings-page`, `theme-cards`, `status-items`, `tab-bar`,
// `switcher`, `onboarding-state`, `diff-header`, `diff-spans`), so a script reads them all with one parser (the end-to-end
// tests' `Readout.swift` reads exactly this):
//
//     kind word… "text"… key="value"… flag…
//
// A line is a kind, then bare words (ids, numbers, flags such as `changed` or `off`), quoted texts (a label, a caption) and
// `key="value"` pairs. A quoted text never holds a straight quote or a line break: they are written `”` and `⏎`.

import Foundation

public enum Readout {
    /// One line: the kind, its bare words, its quoted texts in order, its pairs in order, then its flags.
    public static func line(
        _ kind: String, _ words: [String] = [], quoted: [String] = [], pairs: [(String, String)] = [], flags: [String] = []
    ) -> String {
        let parts = [kind] + words.filter { !$0.isEmpty } + quoted.map(quote) + pairs.map { "\($0.0)=\(quote($0.1))" } + flags.filter { !$0.isEmpty }
        return parts.joined(separator: " ")
    }

    /// A line indented under the one before (a row's caption, a card under its shelf).
    public static func under(
        _ kind: String, _ words: [String] = [], quoted: [String] = [], pairs: [(String, String)] = [], flags: [String] = []
    ) -> String {
        "  " + line(kind, words, quoted: quoted, pairs: pairs, flags: flags)
    }

    /// `"text"`, with the two characters the shape keeps for itself replaced.
    public static func quote(_ text: String) -> String {
        "\"" + text.replacingOccurrences(of: "\"", with: "\u{201D}").replacingOccurrences(of: "\n", with: "\u{23CE}") + "\""
    }

    /// The flag when `on` holds, else nothing (`flags: [Readout.flag("changed", entry.changed)]`).
    public static func flag(_ name: String, _ on: Bool) -> String { on ? name : "" }
}
