// The names a theme's file and its copies get: the theme's own name as the file name (what a person sees in
// Finder), a free "<name> copy" for Save as a copy, and a lower-case dashed one for Export.

import Foundation

enum ThemeFileNames {
    /// `Soltari Night` → `Soltari Night.json`; characters a file name can't hold are dropped.
    static func fileName(for name: String) -> String {
        let clean = name.filter { !"/:\\\0".contains($0) }.trimmingCharacters(in: .whitespaces.union(.init(charactersIn: ".")))
        return (clean.isEmpty ? "Theme" : clean) + ".json"
    }

    /// "Dusk copy", then "Dusk copy 2" and on while `taken`.
    static func copyName(_ name: String, taken: (String) -> Bool) -> String {
        let first = StudioText.copyName(name)
        if !taken(first) { return first }
        return (2...).lazy.map { StudioText.copyName(name, n: $0) }.first { !taken($0) } ?? first
    }

    /// The file name Export suggests: the theme's name in lower case with dashes.
    static func exportName(_ name: String) -> String {
        let stem = name.lowercased().map { $0.isLetter || $0.isNumber ? String($0) : "-" }.joined()
            .split(separator: "-").joined(separator: "-")
        return (stem.isEmpty ? "theme" : stem) + ".json"
    }
}
