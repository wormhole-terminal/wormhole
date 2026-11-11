// The icon theme the rows draw with, as one value that can change: the Files tab, the Changes and History rows and the
// cover over the terminal all read it, and picking another icon theme (or "None") redraws them. It holds whichever
// `FileIconProviding` the app gave it: the core's lookup for the theme in force (`CoreIconTheme`), or the bundled theme
// where there is no core. `nil` is "None": the views draw the system's symbols.

import Foundation
import Observation
import PanelModel

@Observable
public final class FileIconSource: FileIconProviding, @unchecked Sendable {
    /// The one the windows share; the app sets `theme` from the `icon_theme` setting.
    public static let shared = FileIconSource(BundledIconTheme())

    public var theme: (any FileIconProviding)?

    public init(_ theme: (any FileIconProviding)? = nil) { self.theme = theme }

    public func file(_ path: String, dark: Bool) -> URL? { theme?.file(path, dark: dark) }
    public func folder(_ name: String, open: Bool, dark: Bool) -> URL? { theme?.folder(name, open: open, dark: dark) }
    public var rendersInColour: Bool { theme?.rendersInColour ?? true }
}

extension BundledIconTheme {
    /// The file of a bundled icon the core names by key (`wormhole-icons/dark/rust.svg`): in this module's resources.
    public static func resourceURL(forKey key: String) -> URL? {
        guard let rest = key.split(separator: "/", maxSplits: 1).last, key.hasPrefix("wormhole-icons/") else { return nil }
        return Bundle.module.resourceURL?.appendingPathComponent("icons").appendingPathComponent(String(rest))
    }
}
