// A `MenuIcon` as the small template image a menu item carries: an SF Symbol tinted by a theme token, a file's
// icon as Finder draws it, or the generic folder (which asks the system for no path: a folder on a share that
// went away must not make a menu wait).

import AppKit
import PlatformModel

@MainActor
enum MenuIcons {
    static let size = NSSize(width: 16, height: 16)

    static func image(for icon: MenuIcon, style: MenuStyle) -> NSImage? {
        switch icon {
        case .symbol(let name):
            return NSImage(systemSymbolName: name, accessibilityDescription: nil)
        case .file(let path):
            let image = NSWorkspace.shared.icon(forFile: path)
            image.size = size
            return image
        case .folder:
            let image = NSWorkspace.shared.icon(for: .folder)
            image.size = size
            return image
        case .status(let glyph):
            let (name, token) = glyph.symbol
            guard let base = NSImage(systemSymbolName: name, accessibilityDescription: nil) else { return nil }
            guard let color = token.flatMap(style.color) else { return base }
            // A palette symbol keeps its colour in a menu, where a template would be tinted by the text colour.
            return base.withSymbolConfiguration(NSImage.SymbolConfiguration(paletteColors: [color]))
        }
    }
}
