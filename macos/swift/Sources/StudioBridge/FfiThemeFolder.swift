// The person's themes folder through the core's rules: which files are themes, the cap on a read, a save that keeps
// the file it replaces as a saved version first and thins the older ones out, and the saved versions of a theme
// (`studio_folder.rs` over `theme::saved_versions`; the same functions the GPUI Studio is written over). Nothing of
// those rules is written here. Moving a file to the Trash is the platform's, as everywhere else.

import DesignSystem
import Foundation
import StudioModel
import WormholeBindings

public struct FfiThemeFolder: ThemeFolder {
    /// The themes folder, asked at each call: an open Studio's list and new saves follow a profile switch (GPUI's Studio
    /// reads its profile per call); a file already open is written where it is (`home(of:)`).
    private let folder: @Sendable () -> URL
    public var directory: URL { folder() }

    public init(directory: URL) { folder = { directory } }

    public init(folder: @escaping @Sendable () -> URL) { self.folder = folder }

    public func files() -> [URL] { studioFolderFiles(themesDir: directory.path).map { URL(fileURLWithPath: $0) } }

    public func read(_ file: URL) throws -> String { try studioFolderRead(path: file.path) }

    public func write(_ text: String, to file: URL) throws {
        try studioFolderWrite(themesDir: Self.home(of: file), path: file.path, text: text)
    }

    public func versions(of file: URL) -> [SavedVersion] {
        studioFolderVersions(themesDir: Self.home(of: file), path: file.path).map {
            SavedVersion(url: URL(fileURLWithPath: $0.path), saved: Date(timeIntervalSince1970: TimeInterval($0.saved)), pinned: $0.pinned)
        }
    }

    public func readVersion(_ version: SavedVersion) throws -> String { try studioFolderReadVersion(path: version.url.path) }

    public func trash(_ file: URL) throws { try Trash.move(file) }

    /// The themes folder a theme file belongs to: the one holding it. A draft opened before a profile switch is kept, as
    /// in the GPUI Studio, and saved back to its own file; its earlier saves stay beside it, never in the profile switched to.
    private static func home(of file: URL) -> String { file.deletingLastPathComponent().path }
}
