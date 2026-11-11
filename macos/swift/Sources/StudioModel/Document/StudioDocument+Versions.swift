// Versions: the earlier saves of a theme, newest first, the file as it is now on top. Selecting a row previews
// it in every window; Restore loads it as unsaved edits, so it can be undone or saved (`studio/versions.rs`).

import DesignSystem
import Foundation

public struct VersionRow: Sendable, Equatable, Identifiable {
    public let id: URL?
    public let saved: Date?
    public let pinned: Bool
    /// The file, or `nil` when it can't be read.
    public let file: ThemeFile?
    /// The tokens it changed from the save before it (`primary, git.branch +2`), or "first save".
    public let summary: String
}

extension StudioDocument {
    /// How many token names a row lists before "+n".
    static let namesShown = 2

    /// The file as it is now, then the saves behind it, each with what it changed from the one before.
    public func versionRows() -> [VersionRow] {
        guard let url = subject.url else { return [] }
        let folder = environment.folder
        let names = groups.flatMap(\.tokens)
        let older = folder.versions(of: url).map { version in
            (version: version, file: (try? folder.readVersion(version)).flatMap { try? ThemeFile(parsing: $0) })
        }
        let files: [ThemeFile?] = [saved] + older.map(\.file)
        func summary(at index: Int) -> String {
            guard let file = files[index], index + 1 < files.count, let before = files[index + 1] else {
                return index + 1 < files.count ? "" : StudioText.versionsFirst
            }
            return Self.summary(ThemeFile.changedTokens(before, file, among: names))
        }
        let now = VersionRow(id: nil, saved: nil, pinned: false, file: saved, summary: summary(at: 0))
        return [now] + older.enumerated().map { index, entry in
            VersionRow(id: entry.version.url, saved: entry.version.saved, pinned: entry.version.pinned, file: entry.file, summary: summary(at: index + 1))
        }
    }

    /// `primary, git.branch +2`
    static func summary(_ changed: [String]) -> String {
        let shown = changed.prefix(namesShown).joined(separator: ", ")
        return changed.count > namesShown ? "\(shown) +\(changed.count - namesShown)" : shown
    }

    /// Shows a saved version in every window (the file as it is now for `nil`).
    public func preview(_ row: VersionRow) {
        let target = row.id == nil ? file : row.file
        if let target, let resolved = try? workspace.resolve(target, version: shown, increaseContrast: store.increaseContrast) {
            store.preview = resolved
        }
    }

    /// Loads a save as unsaved edits.
    public func restore(_ row: VersionRow) {
        guard let target = row.file, row.id != nil else { return }
        _ = try? take(StudioText.undoRestore) { $0 = target }
    }
}
