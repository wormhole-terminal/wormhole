import Foundation
import Testing

@testable import Wormhole

/// Where the renderer looks for JetBrains Mono: first in the app bundle's resources, where the beta ships it, and only then in
/// the repository (a dev build).
struct ShippedFontsTests {
    /// A resources folder of its own that goes away with the test.
    private func withResources(_ body: (URL) throws -> Void) throws {
        let resources = FileManager.default.temporaryDirectory.appendingPathComponent("wormhole-fonts-test-\(UUID().uuidString)", isDirectory: true)
        try FileManager.default.createDirectory(at: resources, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: resources) }
        try body(resources)
    }

    @Test func theFontsAreFoundWhereTheBundleShipsThem() throws {
        try withResources { resources in
            let folder = resources.appendingPathComponent("Fonts/JetBrainsMono", isDirectory: true)
            try FileManager.default.createDirectory(at: folder, withIntermediateDirectories: true)
            #expect(Options.shippedFonts(in: resources) == nil, "a folder without the regular face is no font folder")
            try Data().write(to: folder.appendingPathComponent("JetBrainsMono-Regular.ttf"))
            #expect(Options.shippedFonts(in: resources) == folder)
        }
    }

    @Test func aBareExecutableOrAnEmptyResourcesFolderShipsNone() throws {
        #expect(Options.shippedFonts(in: nil) == nil)
        try withResources { resources in
            #expect(Options.shippedFonts(in: resources) == nil)
        }
    }
}
