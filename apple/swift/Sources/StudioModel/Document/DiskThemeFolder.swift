// The person's themes folder on disk (`~/Library/Application Support/Wormhole/themes/`). Reads are capped at
// 256 KiB and refuse links, a project's or a download's file being hostile input; writes are atomic; every save
// keeps the file it replaces under `.versions/<id>/<time>.json`, thinned as core does (`VersionPolicy`).

import Foundation

public struct DiskThemeFolder: ThemeFolder {
    public let directory: URL
    static let cap = 256 * 1024

    public init(directory: URL) { self.directory = directory }

    public func files() -> [URL] {
        let names = (try? FileManager.default.contentsOfDirectory(atPath: directory.path)) ?? []
        return names.filter { !$0.hasPrefix(".") && ["json", "jsonc"].contains(($0 as NSString).pathExtension.lowercased()) }
            .sorted { $0.localizedStandardCompare($1) == .orderedAscending }
            .map { directory.appendingPathComponent($0) }
    }

    public func read(_ file: URL) throws -> String {
        try Self.readCapped(file)
    }

    static func readCapped(_ file: URL) throws -> String {
        let values = try file.resourceValues(forKeys: [.isSymbolicLinkKey, .fileSizeKey])
        if values.isSymbolicLink == true { throw ThemeFolderError.link(file.lastPathComponent) }
        if (values.fileSize ?? 0) > cap { throw ThemeFolderError.tooLarge }
        return try String(contentsOf: file, encoding: .utf8)
    }

    public func write(_ text: String, to file: URL) throws {
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        if let previous = try? read(file) { try keep(previous, for: file, at: Date()) }
        try text.write(to: file, atomically: true, encoding: .utf8)
    }

    public func trash(_ file: URL) throws {
        try FileManager.default.trashItem(at: file, resultingItemURL: nil)
    }
}
