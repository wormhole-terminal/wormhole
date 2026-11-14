import Foundation

/// The one place the end-to-end tests start a program: the app under test, `git` for a throwaway repository and the app's own
/// hook client. Nothing else in the suite may call `Process` (`macos/scripts/rules.py` allows it by the marker comments below).
enum Spawn {
    struct Finished {
        var status: Int32
        var output: String
    }

    /// `executable` run to its end with exactly `environment` (never the caller's), its output collected.
    static func run(_ executable: String, _ arguments: [String], environment: [String: String], in directory: URL? = nil) throws -> Finished {
        // process: git, or the app's hook client, for a scenario's setup
        let process = Process()
        process.executableURL = URL(fileURLWithPath: executable)
        process.arguments = arguments
        process.environment = environment
        process.currentDirectoryURL = directory
        process.standardInput = FileHandle.nullDevice
        let pipe = Pipe()
        process.standardOutput = pipe
        process.standardError = pipe
        try process.run()
        let data = pipe.fileHandleForReading.readDataToEndOfFile()
        process.waitUntilExit()
        return Finished(status: process.terminationStatus, output: String(decoding: data, as: UTF8.self))
    }

    /// `executable` started with exactly `environment` and left running, its output in `log`; `onExit` is called once, on another
    /// queue, with its status. For a client of the app that waits on a person's answer (an agent's request), which `run` would block on.
    static func start(
        _ executable: String, _ arguments: [String], environment: [String: String], log: URL, onExit: @escaping @Sendable (Int32) -> Void
    ) throws -> Process {
        FileManager.default.createFile(atPath: log.path, contents: nil, attributes: [.posixPermissions: 0o600])
        let output = try FileHandle(forWritingTo: log)
        // process: the app's own command line client, asking the instance under test through its socket
        let process = Process()
        process.executableURL = URL(fileURLWithPath: executable)
        process.arguments = arguments
        process.environment = environment
        process.standardInput = FileHandle.nullDevice
        process.standardOutput = output
        process.standardError = output
        process.terminationHandler = { finished in
            try? output.close()
            onExit(finished.terminationStatus)
        }
        try process.run()
        return process
    }

    /// The app under test, running, its output in `log`. `onExit` is called once, on another queue, when it has ended.
    static func launch(
        _ executable: URL, arguments: [String], environment: [String: String], log: URL, onExit: @escaping @Sendable (Int32) -> Void
    ) throws -> Process {
        FileManager.default.createFile(atPath: log.path, contents: nil, attributes: [.posixPermissions: 0o600])
        let output = try FileHandle(forWritingTo: log)
        // process: the Wormhole executable under test, as a background instance on scratch folders only
        let process = Process()
        process.executableURL = executable
        process.arguments = arguments
        process.environment = environment
        process.standardInput = FileHandle.nullDevice
        process.standardOutput = output
        process.standardError = output
        process.terminationHandler = { finished in
            try? output.close()
            onExit(finished.terminationStatus)
        }
        try process.run()
        return process
    }
}
