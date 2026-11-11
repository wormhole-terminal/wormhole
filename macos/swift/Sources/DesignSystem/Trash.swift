// The one door to the Trash. A file the app moves away (a theme the person removes) goes through it, never through
// `FileManager.trashItem` directly: a test instance (`--background`) is diverted to a folder in its data folder, so
// it never puts anything into the person's real Trash, and the folder shows what would have gone.

import Foundation
import Synchronization

public enum Trash {
    private static let folder = Mutex<URL?>(nil)

    /// Move trashed files to `folder` from now on (a test instance's), or `nil` for the system's Trash.
    public static func divert(to folder: URL?) { self.folder.withLock { $0 = folder } }

    /// Move `file` to the Trash (or the diverted folder, under a name that keeps clashes apart).
    public static func move(_ file: URL) throws {
        guard let target = folder.withLock({ $0 }) else {
            try FileManager.default.trashItem(at: file, resultingItemURL: nil)
            return
        }
        try FileManager.default.createDirectory(at: target, withIntermediateDirectories: true)
        let unique = "\(UUID().uuidString.prefix(8))-\(file.lastPathComponent)"
        try FileManager.default.moveItem(at: file, to: target.appendingPathComponent(unique))
    }
}
