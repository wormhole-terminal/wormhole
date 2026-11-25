import Foundation
import Testing

@testable import Wormhole

/// The rule that keeps the Swift app out of the 1.x app's data folder, as a function of (bundle identifier, WORMHOLE_DATA_DIR, home):
/// no folder is created, no variable set, no real folder named. The Rust half is `crates/wormhole-ffi/src/isolation.rs`.
struct IsolationTests {
    private static let home = URL(fileURLWithPath: "/Users/someone", isDirectory: true)
    private static let support = "/Users/someone/Library/Application Support"
    private static let beta = "\(support)/Wormhole Beta"
    private static let dev = Isolation.devBundleIdentifier
    private static let shipped = Isolation.betaBundleIdentifier

    /// One row of the table: what the identity gets for the variable, in a launch and in a command line; nil is a refusal.
    struct Row: CustomTestStringConvertible, Sendable {
        var bundle: String?
        var variable: String?
        var launch: String?
        var commandLine: String?
        var testDescription: String { "\(bundle ?? "no bundle") with \(variable.map { "\"\($0)\"" } ?? "no variable")" }
    }

    static let rows: [Row] = [
        // A bundle identifier other than the two is refused, whatever the folder: 1.x's own first.
        Row(bundle: nil, variable: "/tmp/Wormhole-swift-x"),
        Row(bundle: "app.getwormhole.terminal", variable: "/tmp/Wormhole-swift-x"),
        Row(bundle: "app.getwormhole.terminal", variable: nil),
        Row(bundle: "app.getwormhole.terminal.dev", variable: nil),
        Row(bundle: "app.getwormhole.terminal.swift.2", variable: "/tmp/Wormhole-swift-x"),
        Row(bundle: "app.getwormhole.terminal.beta ", variable: nil),
        // The variable names a Wormhole-swift* folder: either identity (a test drives the shipped bundle that way).
        Row(bundle: dev, variable: "/tmp/Wormhole-swift-x", launch: "/tmp/Wormhole-swift-x", commandLine: "/tmp/Wormhole-swift-x"),
        Row(bundle: dev, variable: "/tmp/Wormhole-swift", launch: "/tmp/Wormhole-swift", commandLine: "/tmp/Wormhole-swift"),
        Row(bundle: shipped, variable: "/tmp/Wormhole-swift-e2e", launch: "/tmp/Wormhole-swift-e2e", commandLine: "/tmp/Wormhole-swift-e2e"),
        // Any other folder is refused, 1.x's and the beta's included, for the dev identity.
        Row(bundle: dev, variable: "\(support)/Wormhole"),
        Row(bundle: dev, variable: "\(support)/Wormhole Beta"),
        Row(bundle: dev, variable: "/tmp/Wormhole-dev"),
        Row(bundle: dev, variable: "/tmp/wormhole-swift-x"),
        Row(bundle: dev, variable: "/tmp/Wormhole-Swift-x"),
        Row(bundle: dev, variable: "/tmp/swift"),
        // Without the variable the dev identity is refused for the app and goes to a folder of its own for a command line.
        Row(bundle: dev, variable: nil, launch: nil, commandLine: "\(support)/Wormhole-swift-dev"),
        Row(bundle: dev, variable: "", launch: nil, commandLine: "\(support)/Wormhole-swift-dev"),
        // Without it the beta has its own folder, for both.
        Row(bundle: shipped, variable: nil, launch: beta, commandLine: beta),
        Row(bundle: shipped, variable: "", launch: beta, commandLine: beta),
        // The beta's own folder as a variable is what the app exports for what it starts, which a copy of it inherits: accepted for
        // the beta, as that exact path only.
        Row(bundle: shipped, variable: beta, launch: beta, commandLine: beta),
        Row(bundle: shipped, variable: "\(beta)/", launch: beta, commandLine: beta),
        Row(bundle: shipped, variable: "/elsewhere/Wormhole Beta"),
        Row(bundle: shipped, variable: "\(beta) 2"),
        Row(bundle: shipped, variable: "\(support)/wormhole beta"),
        // 1.x's folder, plainly and behind a `..`, for the beta too.
        Row(bundle: shipped, variable: "\(support)/Wormhole"),
        Row(bundle: shipped, variable: "\(beta)/../Wormhole"),
        Row(bundle: shipped, variable: "/tmp/Wormhole-swift-x/../Wormhole"),
        Row(bundle: dev, variable: "/tmp/Wormhole-swift-x/../Wormhole"),
        Row(bundle: shipped, variable: "/tmp/Wormhole-swift-x/.."),
    ]

    @Test("the rule, row by row", arguments: rows)
    func dataDirectory(row: Row) {
        for (forCommandLine, expected) in [(false, row.launch), (true, row.commandLine)] {
            let found = try? Isolation.dataDirectory(
                bundleIdentifier: row.bundle, variable: row.variable, home: Self.home, forCommandLine: forCommandLine)
            #expect(found?.path == expected, "\(forCommandLine ? "command line" : "launch"): \(String(describing: found?.path))")
        }
    }

    @Test func aRefusalSaysWhatIsWrong() {
        func refusal(_ bundle: String?, _ variable: String?) -> String {
            do {
                _ = try Isolation.dataDirectory(bundleIdentifier: bundle, variable: variable, home: Self.home, forCommandLine: false)
                return "accepted"
            } catch { return "\(error)" }
        }
        #expect(refusal("app.getwormhole.terminal", nil).contains("app.getwormhole.terminal, not"))
        #expect(refusal(nil, nil).contains("missing"))
        #expect(refusal(Self.dev, nil).contains("WORMHOLE_DATA_DIR is not set"))
        #expect(refusal(Self.dev, "\(Self.support)/Wormhole").contains("\(Self.support)/Wormhole is not a folder named Wormhole-swift*"))
    }

    /// A command line has no variable in a terminal's shell (`INSTANCE_VARS`): whatever it is run as, it never lands on 1.x's folder or
    /// fails for the lack of the variable.
    @Test func aCommandLineWithoutTheVariableNeverResolvesToTheRealAppsFolder() throws {
        for bundle in [Self.dev, Self.shipped] {
            for variable in [nil, ""] as [String?] {
                let url = try Isolation.dataDirectory(bundleIdentifier: bundle, variable: variable, home: Self.home, forCommandLine: true)
                #expect(url.lastPathComponent != "Wormhole", "\(bundle): \(url.path)")
                #expect(url.deletingLastPathComponent().path == Self.support)
                #expect(url.lastPathComponent == (bundle == Self.shipped ? "Wormhole Beta" : "Wormhole-swift-dev"))
            }
        }
    }

    @Test func homeIsTheEnvironmentsAndTheSystemsOtherwise() {
        #expect(Isolation.homeDirectory(environment: ["HOME": "/tmp/scratch-home"]).path == "/tmp/scratch-home")
        let system = FileManager.default.homeDirectoryForCurrentUser.path
        #expect(Isolation.homeDirectory(environment: [:]).path == system)
        #expect(Isolation.homeDirectory(environment: ["HOME": ""]).path == system)
    }

    @Test func anIdentityIsItsBundleIdentifier() {
        #expect(Isolation.Identity(bundleIdentifier: Self.dev) == .dev)
        #expect(Isolation.Identity(bundleIdentifier: Self.shipped) == .beta)
        #expect(Isolation.Identity(bundleIdentifier: "app.getwormhole.terminal") == nil)
        #expect(Isolation.Identity(bundleIdentifier: nil) == nil)
        for identity in [Isolation.Identity.dev, .beta] { #expect(Isolation.Identity(bundleIdentifier: identity.bundleIdentifier) == identity) }
    }

    // The constants are the files', which the bundling reads: the two cannot drift apart.

    @Test func theBetaConstantsAreThePackagingFilesAndNeverTheAppsOwn() throws {
        let beta = try Repository.environmentFile("packaging/beta.env")
        let app = try Repository.environmentFile("packaging/app.env")
        #expect(beta["BUNDLE_ID"] == Isolation.betaBundleIdentifier)
        #expect(beta["DATA_DIR_NAME"] == Isolation.betaFolderName)
        #expect(beta["BUNDLE_ID"] != app["BUNDLE_ID"] && beta["DATA_DIR_NAME"] != app["DATA_DIR_NAME"], "the beta installs beside 1.x")
        #expect(Isolation.devBundleIdentifier != app["BUNDLE_ID"] && Isolation.devBundleIdentifier != beta["BUNDLE_ID"])
        #expect(try #require(app["DATA_DIR_NAME"]).hasPrefix(Isolation.folderPrefix) == false, "1.x's folder is not a dev folder")
        #expect(Isolation.devCommandLineFolderName.hasPrefix(Isolation.folderPrefix))
    }

    @Test func theDevIdentityIsTheOneEmbeddedInTheExecutable() throws {
        let plist = try Data(contentsOf: Repository.root.appendingPathComponent("macos/swift/Sources/Wormhole/Info.plist"))
        let info = try #require(try PropertyListSerialization.propertyList(from: plist, format: nil) as? [String: Any])
        #expect(info["CFBundleIdentifier"] as? String == Isolation.devBundleIdentifier)
    }
}

/// The repository's own files, found from this file's place: `macos/swift/Tests/WormholeTests/`.
enum Repository {
    static let root = URL(fileURLWithPath: #filePath).deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()
        .deletingLastPathComponent().deletingLastPathComponent()

    /// A packaging `.env` file as its keys and values, with the quotes of a value dropped (as `peer.rs` and `about/mod.rs` read them).
    static func environmentFile(_ path: String) throws -> [String: String] {
        let text = try String(contentsOf: root.appendingPathComponent(path), encoding: .utf8)
        var values: [String: String] = [:]
        for line in text.split(separator: "\n") where !line.hasPrefix("#") {
            guard let equals = line.firstIndex(of: "=") else { continue }
            values[String(line[..<equals])] = String(line[line.index(after: equals)...]).trimmingCharacters(in: CharacterSet(charactersIn: "\""))
        }
        return values
    }
}
