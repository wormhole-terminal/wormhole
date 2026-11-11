// How a draft is resolved by Rust without touching the person's folder: the draft is written as a theme file
// into a scratch folder and `resolve_theme` is asked for it by name, which is exactly how the app loads a
// theme (so `extends`, versions, comments and every error are core's). The scratch folder also holds copies of
// the person's other themes, so a draft can `extends` one; the file being edited is left out, the draft
// standing in for it.

import DesignSystem
import Foundation

public final class DraftWorkspace: Sendable {
    public let directory: URL
    private let resolver: any ThemeResolving
    private let look: WindowLook

    public init(resolver: any ThemeResolving, folder: any ThemeFolder, editing file: URL?, look: WindowLook) throws {
        self.resolver = resolver
        self.look = look
        directory = FileManager.default.temporaryDirectory.appendingPathComponent("Wormhole-swift-studio-\(UUID().uuidString)")  // text: a scratch folder name
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        for other in folder.files() where other != file {
            if let text = try? folder.read(other) {
                try text.write(to: directory.appendingPathComponent(other.lastPathComponent), atomically: true, encoding: .utf8)
            }
        }
    }

    deinit { try? FileManager.default.removeItem(at: directory) }

    static let draftName = "draft-0.json"

    /// The theme `file` works out to (`version` when it is a pair), in the contrast asked for; throws core's reason.
    public func resolve(_ file: ThemeFile, version: String?, increaseContrast: Bool) throws -> Theme {
        try file.json().write(to: directory.appendingPathComponent(Self.draftName), atomically: true, encoding: .utf8)
        let request = ThemeRequest(
            theme: file.themeName(version: version), themesDirectory: directory, increaseContrast: increaseContrast, look: look)
        return try Theme(resolvedJSON: resolver.resolve(request))
    }
}
