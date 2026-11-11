@testable import DesignSystem
import Foundation
@testable import StudioModel
@testable import StudioUI

/// A themes folder in a scratch directory, a store that follows nothing, and a Studio over both, on the mock resolver.
@MainActor
struct StudioFixtures {
    let folder: DiskThemeFolder
    let store: ThemeStore

    init(themes: [String: String] = [:]) throws {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent("Wormhole-swift-studio-test-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        for (name, text) in themes { try text.write(to: directory.appendingPathComponent(name), atomically: true, encoding: .utf8) }
        folder = DiskThemeFolder(directory: directory)
        store = ThemeStore(follows: false)
        store.systemAppearance = .dark
    }

    var environment: StudioEnvironment { StudioEnvironment(resolver: MockResolver(), folder: folder, knowledge: GalleryKnowledge()) }

    func opening(_ name: String, from file: ThemeFile? = nil) throws -> StudioDocument {
        try StudioDocument.open(theme: name, environment: environment, store: store)
    }

    func remove() { try? FileManager.default.removeItem(at: folder.directory) }
}
