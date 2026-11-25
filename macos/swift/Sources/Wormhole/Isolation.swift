// The Swift app never opens the 1.x app's data (`~/Library/Application Support/Wormhole`). It runs as one of two identities, each
// with a data folder of its own, and says why when it will not start on anything else. The Rust library checks the folder again from
// core's side (`isolation.rs`, the same rule), so a window never opens on the wrong data and a bridge call never reaches it.
//
//              bundle identifier                 data folder
//   dev, test  app.getwormhole.terminal.swift    a folder named Wormhole-swift*, from WORMHOLE_DATA_DIR
//   beta       app.getwormhole.terminal.beta     ~/Library/Application Support/Wormhole Beta
//
// WORMHOLE_DATA_DIR, when set, names a Wormhole-swift* folder for either identity: that is how a test drives the shipped bundle.
// Unset, the beta has its own folder and the dev identity is refused (a build run by hand must say where its data goes). Whatever
// the answer, it is exported as WORMHOLE_DATA_DIR before the first call into the bridge: core derives every path (its data folder,
// the control socket's name) from that variable and falls back to 1.x's own folder without it.

import Foundation

enum Isolation {
    /// What the shipped bundle is called (`BUNDLE_ID` in `packaging/beta.env`, which a test compares with it).
    static let betaBundleIdentifier = "app.getwormhole.terminal.beta"
    /// What a dev build and a test instance are called (`CFBundleIdentifier` of this target's `Info.plist`, embedded in the executable).
    static let devBundleIdentifier = "app.getwormhole.terminal.swift"
    /// What a dev or test data folder is named, as a prefix.
    static let folderPrefix = "Wormhole-swift"
    /// The beta's data folder in Application Support (`DATA_DIR_NAME` in `packaging/beta.env`).
    static let betaFolderName = "Wormhole Beta"
    /// Where a dev build's command line goes without the variable: a place of its own, never 1.x's.
    static let devCommandLineFolderName = "Wormhole-swift-dev"
    private static let variable = "WORMHOLE_DATA_DIR"

    /// Which of the two the app is, by its bundle identifier; anything else is refused.
    enum Identity: Equatable {
        case dev
        case beta

        init?(bundleIdentifier: String?) {
            switch bundleIdentifier {
            case Isolation.devBundleIdentifier: self = .dev
            case Isolation.betaBundleIdentifier: self = .beta
            default: return nil
            }
        }

        var bundleIdentifier: String { self == .beta ? Isolation.betaBundleIdentifier : Isolation.devBundleIdentifier }
    }

    struct Refusal: Error, CustomStringConvertible {
        var description: String
    }

    /// This run's bundle identifier: its identity's, and the dev one for a build that is neither (which `require` has refused).
    static var bundleIdentifier: String { Identity(bundleIdentifier: Bundle.main.bundleIdentifier)?.bundleIdentifier ?? devBundleIdentifier }

    /// The data folder for an identity, from the environment alone: nothing is read, created or exported. Throws when the identity or the
    /// folder is not one the app may open. `forCommandLine`: a command line (`wh`, a hook, `sessions`, `probe`) runs in a terminal's
    /// shell, which never has the variable, so for it the dev identity falls back to a folder of its own instead of being refused.
    static func dataDirectory(bundleIdentifier actual: String?, variable path: String?, home: URL, forCommandLine: Bool) throws -> URL {
        guard let identity = Identity(bundleIdentifier: actual) else {
            throw Refusal(description: "the bundle identifier is \(actual ?? "missing"), not \(devBundleIdentifier) or \(betaBundleIdentifier)")
        }
        let support = home.appendingPathComponent("Library/Application Support", isDirectory: true)
        let beta = support.appendingPathComponent(betaFolderName, isDirectory: true)
        guard let path, !path.isEmpty else {
            switch identity {
            case .beta: return beta
            case .dev where forCommandLine: return support.appendingPathComponent(devCommandLineFolderName, isDirectory: true)
            case .dev: throw Refusal(description: "\(variable) is not set: it must name a folder called \(folderPrefix)*")
            }
        }
        let url = URL(fileURLWithPath: path).standardizedFileURL
        // The beta exports its own folder for what it starts, and a copy of the app that one starts (an update's relaunch) inherits
        // it: that exact folder is the beta's own, so it is no other identity's and no other folder's.
        if identity == .beta, url.path == beta.standardizedFileURL.path { return url }
        guard url.lastPathComponent.hasPrefix(folderPrefix) else {
            throw Refusal(description: "\(url.path) is not a folder named \(folderPrefix)*")
        }
        return url
    }

    /// `$HOME` when it is set and not empty (a test instance sets it to a scratch folder), else the system's: what core takes as home
    /// (`wormhole_term::sys::paths`), so the beta's folder is where core would have put one in Application Support.
    static func homeDirectory(environment: [String: String]) -> URL {
        if let home = environment["HOME"], !home.isEmpty { return URL(fileURLWithPath: home, isDirectory: true) }
        return FileManager.default.homeDirectoryForCurrentUser
    }

    /// The data folder of a launch, created owner-only if it is new and exported as WORMHOLE_DATA_DIR. Throws when the identity or the
    /// folder is not one the app may open.
    @discardableResult
    static func require(
        environment: [String: String] = ProcessInfo.processInfo.environment,
        bundleIdentifier actual: String? = Bundle.main.bundleIdentifier
    ) throws -> URL {
        let url = try dataDirectory(
            bundleIdentifier: actual, variable: environment[variable], home: homeDirectory(environment: environment), forCommandLine: false)
        try FileManager.default.createDirectory(at: url, withIntermediateDirectories: true, attributes: [.posixPermissions: 0o700])
        setenv(variable, url.path, 1)
        return url
    }
}
