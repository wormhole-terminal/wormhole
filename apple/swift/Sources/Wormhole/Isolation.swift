// The experiment never touches the real app's data: it starts only with a bundle identifier of its own and a
// data folder of its own, and says why when it will not. The Rust library checks the folder again from
// core's side (`isolation.rs`); this is the Swift app's half, so a window never opens on the wrong data.

import Foundation

enum Isolation {
    static let bundleIdentifier = "app.getwormhole.terminal.swift"
    static let folderPrefix = "Wormhole-swift"

    struct Refusal: Error, CustomStringConvertible {
        var description: String
    }

    /// The data folder, created owner-only if it is new. Throws when the identity or the folder is not the
    /// experiment's.
    @discardableResult
    static func require(
        environment: [String: String] = ProcessInfo.processInfo.environment,
        bundleIdentifier actual: String? = Bundle.main.bundleIdentifier
    ) throws -> URL {
        guard actual == bundleIdentifier else {
            throw Refusal(description: "the bundle identifier is \(actual ?? "missing"), not \(bundleIdentifier)")
        }
        guard let path = environment["WORMHOLE_DATA_DIR"], !path.isEmpty else {
            throw Refusal(description: "WORMHOLE_DATA_DIR is not set: it must name a folder called \(folderPrefix)*")
        }
        let url = URL(fileURLWithPath: path).standardizedFileURL
        guard url.lastPathComponent.hasPrefix(folderPrefix) else {
            throw Refusal(description: "\(url.path) is not a folder named \(folderPrefix)*")
        }
        try FileManager.default.createDirectory(
            at: url, withIntermediateDirectories: true, attributes: [.posixPermissions: 0o700])
        return url
    }
}
