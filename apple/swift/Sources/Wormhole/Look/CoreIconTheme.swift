// The file icon theme in force, answered by the core's lookup (`theme::icon_theme`: Zed's format, the bundled Wormhole
// Icons or a family of the profile's `icon-themes` folder, light and dark, the case-insensitive fallback, folders by name).
// The core says where an SVG is: a user theme's file, or the bundled theme's key, which is in this app's resources. Each name
// is asked once; a list of a few hundred rows asks for the same few icons on every draw.

#if WITH_UNIFFI
    import FileIconTheme
    import Foundation
    import PanelModel
    import Synchronization
    import WormholeBindings

    final class CoreIconTheme: FileIconProviding, Sendable {
        private let handle: IconThemeHandle
        private let known = Mutex<[String: URL?]>([:])
        let rendersInColour: Bool

        init(directory: String, name: String) {
            handle = IconThemeHandle.open(userDir: directory, name: name)
            rendersInColour = handle.rendersInColour()
        }

        var hasIcons: Bool { handle.hasIcons() }

        func file(_ path: String, dark: Bool) -> URL? {
            remembered("f\(dark ? 1 : 0)\(path)") { handle.file(path: path, dark: dark) }
        }

        func folder(_ name: String, open: Bool, dark: Bool) -> URL? {
            remembered("d\(dark ? 1 : 0)\(open ? 1 : 0)\(name)") { handle.folder(name: name, open: open, dark: dark) }
        }

        private func remembered(_ key: String, _ ask: () -> String?) -> URL? {
            if let hit = known.withLock({ $0[key] }) { return hit }
            let url = ask().flatMap { place in
                place.hasPrefix("wormhole-icons/") ? BundledIconTheme.resourceURL(forKey: place) : URL(fileURLWithPath: place)
            }
            known.withLock { $0[key] = .some(url) }
            return url
        }
    }
#endif
