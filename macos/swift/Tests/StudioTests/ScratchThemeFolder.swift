// A themes folder in a scratch directory for tests that run without the core: it lists, reads and writes theme files and
// keeps every file a save replaces as a version. The rules (caps, links, thinning, naming) are the core's, through
// `FfiThemeFolder`, and are tested with it (`ThemeFFITests`).

import Foundation
@testable import StudioModel

struct ScratchThemeFolder: ThemeFolder {
    let directory: URL

    func files() -> [URL] {
        let names = (try? FileManager.default.contentsOfDirectory(atPath: directory.path)) ?? []
        return names.filter { !$0.hasPrefix(".") && ["json", "jsonc"].contains(($0 as NSString).pathExtension) }.sorted()
            .map { directory.appendingPathComponent($0) }
    }

    func read(_ file: URL) throws -> String { try String(contentsOf: file, encoding: .utf8) }

    func write(_ text: String, to file: URL) throws {
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        if let previous = try? read(file) {
            let folder = versionsFolder(of: file)
            try FileManager.default.createDirectory(at: folder, withIntermediateDirectories: true)
            let count = (try? FileManager.default.contentsOfDirectory(atPath: folder.path).count) ?? 0
            try previous.write(to: folder.appendingPathComponent(String(format: "%04d.json", count)), atomically: true, encoding: .utf8)
        }
        try text.write(to: file, atomically: true, encoding: .utf8)
    }

    func versions(of file: URL) -> [SavedVersion] {
        let folder = versionsFolder(of: file)
        let names = ((try? FileManager.default.contentsOfDirectory(atPath: folder.path)) ?? []).sorted(by: >)
        return names.enumerated().map { SavedVersion(url: folder.appendingPathComponent($1), saved: Date(timeIntervalSince1970: Double(names.count - $0)), pinned: false) }
    }

    func readVersion(_ version: SavedVersion) throws -> String { try read(version.url) }

    func trash(_ file: URL) throws { try FileManager.default.removeItem(at: file) }

    private func versionsFolder(of file: URL) -> URL {
        directory.appendingPathComponent(".versions").appendingPathComponent(file.deletingPathExtension().lastPathComponent)
    }
}
