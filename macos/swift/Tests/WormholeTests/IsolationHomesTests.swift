import Foundation
import Testing

@testable import Wormhole

/// Which of the other programs' folders an instance keeps inside its own data folder: all of them for a dev or test folder, and for
/// the beta's only the one `wh` is installed in (agent status and the Usage tab need the person's real agent folders).
struct IsolationHomesTests {
    private static func confined(_ folder: String) -> [String] {
        Isolation.confinedHomes(for: URL(fileURLWithPath: "/Users/someone/Library/Application Support/\(folder)")).map(\.variable)
    }

    @Test func aDevOrTestFolderKeepsEveryHomeInside() {
        for folder in ["Wormhole-swift-e2e", "Wormhole-swift", "Wormhole-swift-dev"] {
            #expect(Self.confined(folder) == ["WORMHOLE_AGENT_HOME", "WORMHOLE_IMPORT_HOME", "WORMHOLE_CLI_DIR"], "\(folder)")
        }
    }

    @Test func theBetasFolderKeepsTheCommandLineToolAlone() {
        #expect(Self.confined("Wormhole Beta") == ["WORMHOLE_CLI_DIR"])
    }

    /// A folder that is neither is never one the app opens; if it came to that, everything would still be kept inside.
    @Test func anyOtherFolderKeepsEveryHomeInside() {
        for folder in ["Wormhole", "Wormhole Beta 2", "Elsewhere"] { #expect(Self.confined(folder).count == 3, "\(folder)") }
    }

    @Test func eachHomeIsAFolderOfTheDataFolder() {
        let data = URL(fileURLWithPath: "/Users/someone/Library/Application Support/Wormhole-swift-e2e")
        #expect(Isolation.confinedHomes(for: data).map(\.folder) == ["agent-home", "import-home", "cli-bin"])
    }
}
