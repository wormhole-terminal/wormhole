import Foundation

/// Everything one end-to-end scenario may reach, in one folder under `/tmp` (a socket's path must fit in 104 bytes): the app's
/// data folder (named as `Isolation` demands), its control folder, a `HOME`, an agent home, a temp folder and the throwaway
/// repositories. A scenario that restarts the app keeps the same `Scratch`, so the second run finds the first's data.
final class Scratch {
    let root: URL
    let data: URL
    let control: URL
    let home: URL
    let agentHome: URL
    let temp: URL
    let bin: URL
    private(set) var failed = false

    init() throws {
        root = URL(fileURLWithPath: "/tmp/wh-e2e-" + UUID().uuidString.prefix(6).lowercased())
        data = root.appendingPathComponent("Wormhole-swift-e2e")
        control = root.appendingPathComponent("ctl")
        home = root.appendingPathComponent("home")
        agentHome = root.appendingPathComponent("agent-home")
        temp = root.appendingPathComponent("tmp")
        bin = root.appendingPathComponent("bin")
        for folder in [root, data, control, home, agentHome, temp, bin] {
            try FileManager.default.createDirectory(at: folder, withIntermediateDirectories: true, attributes: [.posixPermissions: 0o700])
        }
    }

    func markFailed() { failed = true }

    /// What a program started for the scenario may see of the machine: nothing but system folders and the scratch `HOME`.
    var cleanEnvironment: [String: String] {
        [
            "PATH": "/usr/bin:/bin:/usr/sbin:/sbin", "HOME": home.path, "TMPDIR": temp.path + "/", "LANG": "en_US.UTF-8",
            "GIT_CONFIG_GLOBAL": "/dev/null", "GIT_CONFIG_NOSYSTEM": "1", "GIT_TERMINAL_PROMPT": "0",
        ]
    }

    /// A git repository inside the scratch `HOME` with the given files and one commit; returns its folder.
    @discardableResult
    func repository(_ name: String = "repo", files: [String: String] = ["README.md": "one\n"], message: String = "first commit") throws -> URL {
        let folder = home.appendingPathComponent(name)
        try FileManager.default.createDirectory(at: folder, withIntermediateDirectories: true)
        try git(folder, "init", "-q", "-b", "main")
        for (path, text) in files { try write(folder, path, text) }
        try git(folder, "add", "-A")
        try git(folder, "commit", "-q", "-m", message)
        return folder
    }

    func write(_ folder: URL, _ path: String, _ text: String) throws {
        let url = folder.appendingPathComponent(path)
        try FileManager.default.createDirectory(at: url.deletingLastPathComponent(), withIntermediateDirectories: true)
        try text.write(to: url, atomically: true, encoding: .utf8)
    }

    @discardableResult
    func git(_ folder: URL, _ arguments: String...) throws -> String {
        let identity = ["-c", "user.name=Test", "-c", "user.email=test@example.com", "-c", "commit.gpgsign=false"]
        let done = try Spawn.run("/usr/bin/git", identity + arguments, environment: cleanEnvironment, in: folder)
        if done.status != 0 { throw ScratchError(description: "git \(arguments.joined(separator: " ")) failed: \(done.output)") }
        return done.output
    }

    /// A `.wormhole/` project folder in `folder` with the given files (`actions.json`, `config.json`, `prompts/x.md`, …).
    func project(_ folder: URL, files: [String: String]) throws {
        for (path, text) in files { try write(folder, ".wormhole/" + path, text) }
    }

    /// An empty file whose name is the given bytes, which need not be UTF-8 (Swift's own file calls take text only).
    func rawFile(in folder: URL, named bytes: [UInt8]) throws {
        let path: [CChar] = (Array(folder.path.utf8) + [UInt8(ascii: "/")] + bytes + [0]).map { CChar(bitPattern: $0) }
        let descriptor = open(path, O_CREAT | O_WRONLY, mode_t(0o644))
        guard descriptor >= 0 else { throw ScratchError(description: "cannot make a file with a raw name in \(folder.path)") }
        close(descriptor)
    }

    /// A script named `name` that reads what is typed and runs nothing: a terminal whose program is it is taken for that agent
    /// (`sh codex` is Codex, as `HookBridgeTests` does it). Not a copy of `/bin/cat`: macOS ends a copied system binary at once.
    func agentShell(named name: String) throws -> URL {
        let script = bin.appendingPathComponent(name)
        if !FileManager.default.fileExists(atPath: script.path) {
            try Data("#!/bin/sh\nwhile IFS= read -r line; do :; done\n".utf8).write(to: script)
            try FileManager.default.setAttributes([.posixPermissions: 0o755], ofItemAtPath: script.path)
        }
        return script
    }

    /// Where the app's session hosts keep their sockets and meta files (core `ipc::hosts::hosts_dir`).
    var hostsFolder: URL { data.appendingPathComponent("hosts") }

    /// Persistent terminals for this run: the built session host linked into the scratch `bin` (the app is told where by
    /// `WORMHOLE_HOST_BIN`, as `HostingSetup` reads it; the build's own folder is never touched) and the feature's flag on in
    /// the scratch `config.json`. The link's path, for the app's environment.
    func giveHosts(from built: URL) throws -> URL {
        let link = bin.appendingPathComponent("wormhole-host")
        if !FileManager.default.fileExists(atPath: link.path) { try FileManager.default.createSymbolicLink(at: link, withDestinationURL: built) }
        try Data(#"{"feature_flags": {"unlocked": true, "persistent_terminals": true}}"#.utf8).write(to: data.appendingPathComponent("config.json"))
        return link
    }

    /// The session hosts of `folder` whose process runs, by id: each host's meta file (`<id>.meta`) names its process
    /// (`pid=`), which is asked whether it is alive. A host that died leaves its file and is not counted.
    static func liveHosts(in folder: URL) -> [String: pid_t] {
        let names = (try? FileManager.default.contentsOfDirectory(atPath: folder.path)) ?? []
        var live: [String: pid_t] = [:]
        for name in names where name.hasSuffix(".meta") {
            guard let text = try? String(contentsOf: folder.appendingPathComponent(name), encoding: .utf8),
                let line = text.split(separator: "\n").first(where: { $0.hasPrefix("pid=") }),
                let pid = pid_t(line.dropFirst(4)), pid > 0, kill(pid, 0) == 0
            else { continue }
            live[String(name.dropLast(5))] = pid
        }
        return live
    }

    func liveHosts() -> [String: pid_t] { Self.liveHosts(in: hostsFolder) }

    /// The text of the app's data folder file `name`, nil when it is not there.
    func dataFile(_ name: String) -> String? { try? String(contentsOf: data.appendingPathComponent(name), encoding: .utf8) }

    /// Copies what a failure needs (the control files, the app's output and its `config.json`) to `$WORMHOLE_E2E_ARTIFACTS`, when set.
    func keepArtifacts(as label: String) {
        guard let target = ProcessInfo.processInfo.environment["WORMHOLE_E2E_ARTIFACTS"], !target.isEmpty else { return }
        let folder = URL(fileURLWithPath: target).appendingPathComponent(label + "-" + root.lastPathComponent)
        try? FileManager.default.createDirectory(at: folder, withIntermediateDirectories: true)
        let names = (try? FileManager.default.contentsOfDirectory(atPath: control.path)) ?? []
        for name in names where name.hasPrefix("out") || name.hasPrefix("cmd") {
            try? FileManager.default.copyItem(at: control.appendingPathComponent(name), to: folder.appendingPathComponent(name))
        }
        try? FileManager.default.copyItem(at: root.appendingPathComponent("app.log"), to: folder.appendingPathComponent("app.log"))
        try? FileManager.default.copyItem(at: data.appendingPathComponent("config.json"), to: folder.appendingPathComponent("config.json"))
        let logs = data.appendingPathComponent("logs")
        try? FileManager.default.copyItem(at: logs, to: folder.appendingPathComponent("logs"))
    }

    /// The scratch folder and everything in it (the repositories, the data, the trash seam) goes.
    func remove() { try? FileManager.default.removeItem(at: root) }
}

struct ScratchError: Error, CustomStringConvertible {
    var description: String
}
