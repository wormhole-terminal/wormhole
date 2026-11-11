// Saved versions on disk: `.versions/<id>/2026-10-29T14-03-07.json`, a name core reads as the same time, with
// `-n` for a second save in one second and `.original` for the pinned first file of a hand-written theme.

import Foundation

extension DiskThemeFolder {
    private static let stamp: DateFormatter = {
        let format = DateFormatter()
        format.locale = Locale(identifier: "en_US_POSIX")
        format.timeZone = TimeZone(identifier: "UTC")  // text: a time zone identifier
        format.dateFormat = "yyyy-MM-dd'T'HH-mm-ss"
        return format
    }()

    /// A theme's folder name: its file's stem as a slug (`My Night.json` → `my_night`).
    static func id(of file: URL) -> String {
        let stem = file.deletingPathExtension().lastPathComponent.lowercased()
        let parts = stem.split { !($0.isLetter || $0.isNumber) }
        return parts.joined(separator: "_")
    }

    func versionsDirectory(for file: URL) -> URL {
        directory.appendingPathComponent(".versions").appendingPathComponent(Self.id(of: file))
    }

    public func versions(of file: URL) -> [SavedVersion] {
        let folder = versionsDirectory(for: file)
        guard (try? folder.resourceValues(forKeys: [.isSymbolicLinkKey]).isSymbolicLink) != true else { return [] }
        let names = (try? FileManager.default.contentsOfDirectory(atPath: folder.path)) ?? []
        return names.compactMap { name -> SavedVersion? in
            guard name.hasSuffix(".json"), name.count >= 24, let date = Self.stamp.date(from: String(name.prefix(19))) else { return nil }
            return SavedVersion(url: folder.appendingPathComponent(name), saved: date, pinned: name.hasSuffix(".original.json"))
        }.sorted { $0.saved == $1.saved ? $0.url.path > $1.url.path : $0.saved > $1.saved }
    }

    public func readVersion(_ version: SavedVersion) throws -> String { try Self.readCapped(version.url) }

    func keep(_ previous: String, for file: URL, at now: Date) throws {
        let folder = versionsDirectory(for: file)
        if (try? folder.resourceValues(forKeys: [.isSymbolicLinkKey]).isSymbolicLink) == true {
            throw ThemeFolderError.link(folder.lastPathComponent)
        }
        try FileManager.default.createDirectory(at: folder, withIntermediateDirectories: true)
        let existing = versions(of: file)
        let handWritten = (try? ThemeFile(parsing: previous))?.root["version"] == nil
        let pinned = handWritten && !existing.contains { $0.pinned }
        let base = Self.stamp.string(from: now)
        let name = (0...).lazy.map { "\(base)\($0 > 0 ? "-\($0)" : "")\(pinned ? ".original" : "").json" }  // text: a file name
            .first { !FileManager.default.fileExists(atPath: folder.appendingPathComponent($0).path) } ?? base
        try previous.write(to: folder.appendingPathComponent(name), atomically: true, encoding: .utf8)
        let all = versions(of: file)
        let keep = VersionPolicy.keep(all.map { ($0.saved, $0.pinned) }, now: now)
        for (version, keeps) in zip(all, keep) where !keeps { try? FileManager.default.removeItem(at: version.url) }
    }
}
