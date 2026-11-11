// Throwaway repositories for the tests, made with the system's git (the product code never spawns it) and kept
// under a temp folder of their own. The FFI refuses to run unless the data folder is the experiment's.

import Foundation
import PanelModel

@testable import GitBridge

enum Repo {
    static let dataDir: Void = {
        setenv("WORMHOLE_DATA_DIR", NSTemporaryDirectory() + "Wormhole-swift-h-git-tests", 1)
    }()

    static func backends(
        allowed: @escaping @Sendable (String) async -> String? = { _ in nil },
        created: @escaping @Sendable (MadeWorktree, [String: String]) async -> Void = { _, _ in }
    ) throws -> GitBackends {
        _ = dataDir
        return try GitBridge.makeBackends(worktreeAllowed: allowed, worktreeCreated: created)
    }

    /// A repository with one commit holding `files`; the folder is removed with `cleanup()`.
    static func make(_ files: [String: String] = ["a.txt": "1\n2\n3\n"]) throws -> URL {
        _ = dataDir
        let root = URL(fileURLWithPath: NSTemporaryDirectory())
            .appendingPathComponent("wormhole-git-bridge-\(UUID().uuidString)", isDirectory: true)
            .resolvingSymlinksInPath()
        try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
        try git(root, "init", "-q")
        try git(root, "config", "core.excludesFile", "/dev/null")
        for (path, text) in files { try write(root, path, text) }
        try git(root, "add", "-A")
        try git(root, "commit", "-q", "-m", "initial")
        return root
    }

    static func write(_ root: URL, _ path: String, _ text: String) throws {
        let url = root.appendingPathComponent(path)
        try FileManager.default.createDirectory(at: url.deletingLastPathComponent(), withIntermediateDirectories: true)
        try text.write(to: url, atomically: true, encoding: .utf8)
    }

    @discardableResult
    static func git(_ root: URL, _ arguments: String...) throws -> String {
        let process = Process()
        process.executableURL = URL(fileURLWithPath: "/usr/bin/env")
        process.arguments =
            ["git", "-c", "user.name=Test", "-c", "user.email=test@example.com", "-c", "init.defaultBranch=main"]
            + arguments
        process.currentDirectoryURL = root
        process.environment = ["PATH": "/usr/bin:/bin:/usr/local/bin:/opt/homebrew/bin", "GIT_CONFIG_GLOBAL": "/dev/null", "GIT_CONFIG_NOSYSTEM": "1"]
        let out = Pipe()
        process.standardOutput = out
        process.standardError = FileHandle.nullDevice
        try process.run()
        let data = out.fileHandleForReading.readDataToEndOfFile()
        process.waitUntilExit()
        return String(decoding: data, as: UTF8.self)
    }

    static func cleanup(_ root: URL) { try? FileManager.default.removeItem(at: root) }
}
