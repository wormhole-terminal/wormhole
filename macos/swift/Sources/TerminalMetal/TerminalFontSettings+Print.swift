// The terminal's own face on paper: Print sets the text in the font Settings chose, not the system's.

import AppKit
import CoreText

extension TerminalFontSettings {
    /// The regular face at `size` points, for text a person reads on paper or in a document. Falls back to the
    /// system's monospaced font where the family or file cannot be found.
    @MainActor
    public func printFont(size: CGFloat) -> NSFont {
        let found: NSFont? =
            switch font {
            case .system(let family): NSFont(name: family, size: size)
            case .files(let regular, _, _, _):
                (CTFontManagerCreateFontDescriptorsFromURL(regular as CFURL) as? [CTFontDescriptor])?.first
                    .map { CTFontCreateWithFontDescriptor($0, size, nil) as NSFont }
            }
        return found ?? .monospacedSystemFont(ofSize: size, weight: .regular)
    }
}
