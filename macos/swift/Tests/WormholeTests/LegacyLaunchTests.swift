import Foundation
import Testing

@testable import Wormhole

/// What a launch brings forward from before the rename rewrites agent hooks and takes a role from the 1.x app: it applies to the 1.x
/// app's folder and to no other, the beta's and the dev folders included.
struct LegacyLaunchTests {
    private static let support = "/Users/someone/Library/Application Support"

    @Test(
        "applies to a folder named exactly Wormhole and to no other",
        arguments: [
            ("Wormhole", true),
            ("Wormhole Beta", false), ("Wormhole-swift", false), ("Wormhole-swift-e2e", false), ("Wormhole-swift-dev", false),
            ("Wormhole-dev", false), ("Wormhole ", false), ("wormhole", false), ("Wormhole2", false), ("Andromeda", false),
        ])
    func applies(name: String, expected: Bool) {
        #expect(LegacyLaunch.applies(to: URL(fileURLWithPath: "\(Self.support)/\(name)")) == expected)
    }

    @Test func theFolderIsTheOneThe1xAppKeepsItsDataIn() throws {
        #expect(try Repository.environmentFile("packaging/app.env")["DATA_DIR_NAME"] == LegacyLaunch.dataFolderName)
    }

    /// Whatever folder the rule hands the app, the legacy upgrades (hooks, default terminal) stay away from it: the rule never
    /// returns 1.x's, so `applies` is false for every answer it can give.
    @Test func legacyUpgradesNeverApplyToAFolderTheAppCanOpen() throws {
        let home = URL(fileURLWithPath: "/Users/someone", isDirectory: true)
        for bundle in [Isolation.devBundleIdentifier, Isolation.betaBundleIdentifier] {
            for variable in [nil, "", "/tmp/Wormhole-swift-x", "\(Self.support)/Wormhole Beta"] as [String?] {
                for forCommandLine in [false, true] {
                    guard let folder = try? Isolation.dataDirectory(bundleIdentifier: bundle, variable: variable, home: home, forCommandLine: forCommandLine)
                    else { continue }
                    #expect(!LegacyLaunch.applies(to: folder), "\(bundle) with \(variable ?? "no variable"): \(folder.path)")
                }
            }
        }
    }
}
