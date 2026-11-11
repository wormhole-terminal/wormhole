// What every test here works in: a temp folder named as the experiment's data folder must be, so the library
// accepts it, and nothing outside it is touched.

import Foundation

struct Scratch {
    let root: URL
    /// A data folder (`Wormhole-swift*`) inside `root`.
    let data: URL

    init() throws {
        root = FileManager.default.temporaryDirectory.appendingPathComponent("aux-bridge-\(UUID().uuidString.prefix(8))")
        data = root.appendingPathComponent("Wormhole-swift-t")
        try FileManager.default.createDirectory(at: data, withIntermediateDirectories: true)
    }

    func folder(_ path: String) throws -> URL {
        let url = root.appendingPathComponent(path)
        try FileManager.default.createDirectory(at: url, withIntermediateDirectories: true)
        return url
    }

    func write(_ text: String, to path: String) throws {
        let url = root.appendingPathComponent(path)
        try FileManager.default.createDirectory(at: url.deletingLastPathComponent(), withIntermediateDirectories: true)
        try text.write(to: url, atomically: true, encoding: .utf8)
    }

    func remove() { try? FileManager.default.removeItem(at: root) }
}
