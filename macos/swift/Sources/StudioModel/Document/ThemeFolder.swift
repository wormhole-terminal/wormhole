// What the Studio asks of the themes folder, as a protocol: list the person's theme files, read and write one,
// keep the file a save replaces as a version, and trash one. The real folder is `FfiThemeFolder` (StudioBridge):
// the rules about names, caps, links and thinning are core's (`storage::files`, `theme::saved_versions`).

import Foundation

public struct SavedVersion: Sendable, Equatable, Identifiable {
    public let url: URL
    public let saved: Date
    /// The original hand-written file: never thinned out.
    public let pinned: Bool
    public var id: URL { url }

    public init(url: URL, saved: Date, pinned: Bool) { (self.url, self.saved, self.pinned) = (url, saved, pinned) }
}

public protocol ThemeFolder: Sendable {
    var directory: URL { get }
    /// The theme files in the folder (`.json` and `.jsonc`), by name.
    func files() -> [URL]
    func read(_ file: URL) throws -> String
    /// Writes atomically, keeping the file it replaces as a saved version first.
    func write(_ text: String, to file: URL) throws
    func versions(of file: URL) -> [SavedVersion]
    func readVersion(_ version: SavedVersion) throws -> String
    func trash(_ file: URL) throws
}

public enum ThemeFolderError: Error, Equatable, LocalizedError {
    case tooLarge, link(String)

    public var errorDescription: String? {
        switch self {
        case .tooLarge: StudioText.folderTooLarge
        case .link(let name): StudioText.folderLink(name)
        }
    }
}
