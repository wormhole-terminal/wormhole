import Foundation
import Synchronization
import Testing

/// Every end-to-end scenario is a test of this one suite, which runs them one at a time: an instance is a real app with real
/// terminals. It skips itself, with the reason, when the app is not built (`swift build` with the bridge linked).
@Suite(.serialized, .enabled(if: Instance.executable() != nil, "the Wormhole app is not built: run `swift build` in macos/swift with the bridge linked"))
struct EndToEnd {}

/// One run of the built app as a test instance (`--background`, scratch folders only), driven through its control folder
/// (`macos/parity/DRIVING.md`). A batch of commands is done when the app answers the `sync` line written after each of them:
/// the wait is on the output file's change events and a deadline, never a pause. `with` always ends what it started.
@MainActor
final class Instance {
    enum Shell {
        /// Typed text is echoed; nothing is ever executed.
        case cat
        /// `/bin/zsh -f`: a real shell without startup files, in the scratch `HOME`.
        case zsh
        /// A script named like an agent that only reads what is typed, so the terminal is taken for that agent.
        case agent(String)
        /// No `WORMHOLE_SHELL`: the app runs what its settings and a terminal's choice say (`default_shell`, a shell picked
        /// for one terminal, a trusted project's). A scenario with it names a system shell in `config` first.
        case configured
    }

    let scratch: Scratch
    private let shell: Shell
    /// What the owner's approval answers (`WORMHOLE_AUTH`): `cancel`, or `allow` for a scenario that trusts a project. Never
    /// a prompt either way.
    private let auth: String
    private let label: String
    /// Programs named like coding agents the app finds on its agent path (`WORMHOLE_AGENT_PATH`, a scratch folder of their
    /// own): empty, none is found, as on a Mac with no agent.
    private var agents: [String] = []
    /// The app itself: what `cli` runs as a client of the instance.
    let program: URL
    /// The session host the app is given (`Scratch.giveHosts`), nil for a run whose terminals stay in the app.
    private var host: URL?
    private var process: Process?
    private let ended = Ending()
    private var batches = 0
    private var attached = false
    private(set) var failed = false

    private init(scratch: Scratch, shell: Shell, auth: String, label: String, program: URL) {
        (self.scratch, self.shell, self.auth, self.label, self.program) = (scratch, shell, auth, label, program)
    }

    /// The built app: `$WORMHOLE_E2E_APP`, else the package's `Wormhole` product beside the test bundle or in `.build`.
    nonisolated static func executable() -> URL? {
        let files = FileManager.default
        if let path = ProcessInfo.processInfo.environment["WORMHOLE_E2E_APP"], files.isExecutableFile(atPath: path) { return URL(fileURLWithPath: path) }
        var candidates = Bundle.allBundles.filter { $0.bundleURL.pathExtension == "xctest" }
            .map { $0.bundleURL.deletingLastPathComponent().appendingPathComponent("Wormhole") }
        // This file is macos/swift/Tests/EndToEndTests/Instance.swift.
        let package = URL(fileURLWithPath: #filePath).deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()
        candidates += [".build/debug/Wormhole", ".build/release/Wormhole"].map { package.appendingPathComponent($0) }
        return candidates.first { files.isExecutableFile(atPath: $0.path) }
    }

    /// The built session host: `$WORMHOLE_E2E_HOST`, else the debug build in the bridge's own target folder, where the
    /// workflow's "Session host for the hosting tests" puts it (`cargo build -p wormhole-host`).
    nonisolated static func hostProgram() -> URL? {
        let files = FileManager.default
        if let path = ProcessInfo.processInfo.environment["WORMHOLE_E2E_HOST"], files.isExecutableFile(atPath: path) { return URL(fileURLWithPath: path) }
        let package = URL(fileURLWithPath: #filePath).deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()
        let built = package.appendingPathComponent("../../crates/wormhole-ffi/target/debug/wormhole-host").standardizedFileURL
        return files.isExecutableFile(atPath: built.path) ? built : nil
    }

    /// Starts an instance, runs `body`, and ends the instance and removes its scratch folder, whatever `body` does. `auth` is what
    /// the owner's question answers (a project's trust, a Probe's approval); `config` is the text of the default profile's
    /// `config.json` before the first launch (a flag or a mode the app reads at launch); `hosts`: Persistent terminals on, with
    /// the built session host (`hostProgram`), which writes that file itself, so the two are not given together.
    static func with(
        shell: Shell = .cat, auth: String = "cancel", config: String? = nil, hosts: Bool = false, agents: [String] = [], label: String = #function,
        _ body: @MainActor (Instance) async throws -> Void
    ) async throws {
        guard let program = executable() else { throw ScratchError(description: "the Wormhole app is not built") }
        let app = Instance(scratch: try Scratch(), shell: shell, auth: auth, label: label, program: program)
        app.agents = agents
        // The default profile's files are the data folder's own.
        if let config {
            guard !hosts else {
                app.scratch.remove()
                throw ScratchError(description: "`config` and `hosts` together: the hosts' flag is written into config.json, put it in `config`")
            }
            do { try Data(config.utf8).write(to: app.scratch.data.appendingPathComponent("config.json")) } catch {
                app.scratch.remove()
                throw error
            }
        }
        if hosts {
            guard let built = hostProgram() else { throw ScratchError(description: "no session host is built: cargo build -p wormhole-host") }
            app.host = try app.scratch.giveHosts(from: built)
        }
        do {
            try await app.start()
            try await body(app)
        } catch {
            app.fail("\(error)")
            await app.finish()
            throw error
        }
        await app.finish()
    }

    // MARK: - Starting and ending

    private func environment() throws -> [String: String] {
        var environment = scratch.cleanEnvironment
        let program: URL?
        switch shell {
        case .cat: program = URL(fileURLWithPath: "/bin/cat")
        case .agent(let name): program = try scratch.agentShell(named: name)
        case .configured: program = nil
        case .zsh:
            let zsh = scratch.bin.appendingPathComponent("zsh-f")
            try Data("#!/bin/zsh\nexec /bin/zsh -f \"$@\"\n".utf8).write(to: zsh)
            try FileManager.default.setAttributes([.posixPermissions: 0o755], ofItemAtPath: zsh.path)
            program = zsh
        }
        environment["WORMHOLE_DATA_DIR"] = scratch.data.path
        environment["WORMHOLE_DEVCTL"] = scratch.control.path
        if let program { environment["WORMHOLE_SHELL"] = program.path }
        environment["WORMHOLE_AUTH"] = auth
        environment["WORMHOLE_NOTIFY"] = "log"
        environment["WORMHOLE_BACKGROUND"] = "1"
        environment["WORMHOLE_AGENT_HOME"] = scratch.agentHome.path
        environment["WORMHOLE_AGENT_PATH"] = agents.isEmpty ? "" : try scratch.agentPath(agents).path
        if let host { environment["WORMHOLE_HOST_BIN"] = host.path }
        return environment
    }

    /// The window's content in points: both columns open leave the terminal 874 points, 100 columns at the default font,
    /// and room to split it.
    static let window = (width: 1600, height: 900)

    /// Launches the app on this scratch folder, waits until its window answers and gives it the suite's size.
    func start() async throws {
        ended.reset()
        if !FileManager.default.fileExists(atPath: outFile(nil).path) { FileManager.default.createFile(atPath: outFile(nil).path, contents: nil) }
        let ended = ended
        process = try Spawn.launch(
            program, arguments: ["--background"], environment: try environment(), log: scratch.root.appendingPathComponent("app.log"),
            onExit: { ended.finish($0) })
        let said = await until("state", timeout: 40) { $0.contains("terminals=") }
        expect(said.contains("terminals="), "the app did not come up with a window")
        // One window size for every run, whatever display it runs on (a runner's is smaller than the window): the app keeps
        // a size asked through its control folder beyond the screen's edge. Checked, so a display that still decides fails here.
        let size = "(\(Self.window.width).0, \(Self.window.height).0)"
        await send(["size \(Self.window.width)x\(Self.window.height)"])
        let sized = await until("geometry") { $0.contains("content=\(size)") }
        expect(sized.contains("content=\(size)"), "the window did not take the size \(size):\n\(sized)")
        // The panes have followed too: together they are as wide as the window less its two side columns, never the 304
        // points a small display left them. A window that was resized but not laid out fails here, saying what it had.
        let room = Double(Self.window.width) - 760
        let laidOut: (String) -> Bool = { said in
            said.split(separator: "\n").filter { $0.hasPrefix("pane ") }.map { $0.split(separator: " ").compactMap { Double($0) } }
                .contains { $0.count >= 5 && $0[1] + $0[3] >= room }
        }
        let panes = await until("panes", laidOut)
        if !laidOut(panes) { fail("the window took the size but its panes were not laid out:\n\(panes)\n\(await keysAndFrame())") }
    }

    /// Quits as ⌘Q does without the question (the terminals' state is saved first) and starts the app again on the same folders.
    func restart() async throws {
        await quit(command: "quit")
        try await start()
    }

    /// Ends the app by a signal, as a crash (`SIGKILL`: nothing is saved) or `kill <pid>` (`SIGTERM`: it quits as it decides),
    /// and waits until it has ended: whether it did (it is then ended with `SIGKILL`). `start()` launches it again on the same folders.
    @discardableResult
    func end(by signal: Int32, timeout: TimeInterval = 20) async -> Bool {
        guard let process, !ended.isDone else { return false }
        kill(process.processIdentifier, signal)
        if await waitForExit(timeout: timeout) { return true }
        fail("the app did not end on signal \(signal): ending it with SIGKILL")
        kill(process.processIdentifier, SIGKILL)
        _ = await waitForExit(timeout: 10)
        return false
    }

    /// Sends `quit`, `quit-keep` or `quit-end-all` and waits until the app has ended (by its pid when it does not).
    func quit(command: String) async {
        guard let process, !ended.isDone else { return }
        // Staged and renamed, as every batch is: the app takes `cmd` the moment it appears, and an empty one is a lost command.
        let staged = scratch.control.appendingPathComponent("cmd.tmp")
        try? Data((command + "\n").utf8).write(to: staged)
        rename(staged.path, scratch.control.appendingPathComponent("cmd").path)
        if await waitForExit(timeout: 20) { return }
        fail("the app did not end after `\(command)`: ending it by its pid")
        kill(process.processIdentifier, SIGTERM)
        if await waitForExit(timeout: 10) { return }
        kill(process.processIdentifier, SIGKILL)
        _ = await waitForExit(timeout: 10)
    }

    /// Ends the app (and any session host: End All Terminals), checks that no host of its folder runs on, keeps a failure's
    /// files, removes the scratch folder.
    func finish() async {
        if !ended.isDone { await quit(command: "quit-end-all") }
        let left = await hostsLeft()
        expect(left.isEmpty, "a session host was left running after Quit and End All: \(left.keys.sorted())")
        // Never left behind by a failed run: the hosts it started end with it.
        for pid in left.values { kill(pid, SIGTERM) }
        if failed { scratch.keepArtifacts(as: label.replacingOccurrences(of: "()", with: "")) }
        scratch.remove()
    }

    // MARK: - Driving

    /// `out.txt` answers `cmd`, which acts on the window in front (the last one opened or used, in a background instance);
    /// `out<n>.txt` answers `cmd<n>`, which acts on the n-th window whatever is in front (`DevCtl.swift`, `AppController.front`).
    func outFile(_ window: Int?) -> URL { scratch.control.appendingPathComponent(window.map { "out\($0).txt" } ?? "out.txt") }

    /// Runs the commands in order in the window's control file (`cmd`, or `cmd<n>` for the n-th window); the answer to each, in the same order.
    @discardableResult
    func send(_ commands: [String], window: Int? = nil, timeout: TimeInterval = 30) async -> [String] {
        batches += 1
        let token = "e2e\(batches)"
        let files = FileManager.default
        let out = outFile(window)
        if !files.fileExists(atPath: out.path) { files.createFile(atPath: out.path, contents: nil) }
        let before = (try? Data(contentsOf: out).count) ?? 0
        let marks = commands.indices.map { "sync \(token).\($0)" }
        let batch = zip(commands, marks).flatMap { [$0, $1] }.joined(separator: "\n") + "\n"
        let command = scratch.control.appendingPathComponent(window.map { "cmd\($0)" } ?? "cmd")
        let staged = scratch.control.appendingPathComponent("cmd.tmp")
        do {
            try Data(batch.utf8).write(to: staged)
            rename(staged.path, command.path)
        } catch {
            fail("cannot write the control file: \(error)")
            return commands.map { _ in "" }
        }
        let last = marks.last ?? ""
        let done = await watch(out, timeout: timeout) { Self.lines(of: out, after: before).contains(last) }
        let lines = Self.lines(of: out, after: before)
        if !done { fail("no answer to `\(commands.joined(separator: "; "))` in \(Int(timeout)) s (the app \(ended.isDone ? "has ended" : "is still running"))") }
        var answers: [String] = [], current: [String] = []
        for line in lines {
            if marks.contains(line) {
                answers.append(current.joined(separator: "\n"))
                current = []
            } else {
                current.append(line)
            }
        }
        answers.append(current.joined(separator: "\n"))
        return Array(answers.prefix(commands.count)) + Array(repeating: "", count: max(0, commands.count - answers.count))
    }

    func reply(_ command: String, window: Int? = nil) async -> String { (await send([command], window: window)).first ?? "" }

    /// Asks again, a pause of the app's own (`delay`) between the asks, until the answer satisfies `done` or the deadline passes.
    /// For facts nothing announces (an agent seen by its program's name, a repository read after a change).
    @discardableResult
    func until(_ command: String, window: Int? = nil, timeout: TimeInterval = 20, _ done: (String) -> Bool) async -> String {
        let start = Date()
        var answer = ""
        repeat {
            answer = (await send([command, "delay 150"], window: window)).first ?? ""
            if done(answer) { return answer }
        } while Date().timeIntervalSince(start) < timeout && !ended.isDone
        fail("timed out waiting on `\(command)`; the last answer was:\n\(answer)")
        return answer
    }

    func rows(window: Int? = nil) async -> Listing { Listing(await reply("rows", window: window)) }

    // MARK: - Waiting

    nonisolated private static func lines(of file: URL, after offset: Int) -> [String] {
        guard let data = try? Data(contentsOf: file), data.count >= offset else { return [] }
        return String(decoding: data.dropFirst(offset), as: UTF8.self).split(separator: "\n", omittingEmptySubsequences: false).map(String.init)
    }

    /// Waits for `done` to hold, checking it when `url` changes (a file's write or a folder's entries) and once at the start;
    /// false when the deadline passes first or (unless `pastEnd`) the app has ended.
    func watch(_ url: URL, timeout: TimeInterval, pastEnd: Bool = false, _ done: @escaping @Sendable () -> Bool) async -> Bool {
        let descriptor = open(url.path, O_EVTONLY)
        guard descriptor >= 0 else { return done() }
        let (stream, events) = AsyncStream<Void>.makeStream(bufferingPolicy: .bufferingNewest(1))
        let source = DispatchSource.makeFileSystemObjectSource(
            fileDescriptor: descriptor, eventMask: [.write, .extend, .delete, .rename], queue: .global())
        // The handlers run on a global queue, not on the main actor this method is on: `@Sendable` says so (unsaid, the
        // runtime's isolation check ends the test process).
        source.setEventHandler { @Sendable in events.yield() }
        source.setCancelHandler { @Sendable in close(descriptor) }
        source.resume()
        let deadline = DispatchWorkItem { @Sendable in events.finish() }
        DispatchQueue.global().asyncAfter(deadline: .now() + timeout, execute: deadline)
        if !pastEnd { ended.onEnd { @Sendable in events.finish() } }
        defer {
            source.cancel()
            deadline.cancel()
            ended.onEnd(nil)
        }
        events.yield()
        for await _ in stream where done() { return true }
        return done()
    }

    /// The session hosts of this run's folder still running once the hosts folder has settled (a host that was asked to end
    /// takes a moment, its shell's hang-up included).
    func hostsLeft(timeout: TimeInterval = 10) async -> [String: pid_t] {
        let folder = scratch.hostsFolder
        guard FileManager.default.fileExists(atPath: folder.path) else { return [:] }
        _ = await watch(folder, timeout: timeout, pastEnd: true) { Scratch.liveHosts(in: folder).isEmpty }
        return Scratch.liveHosts(in: folder)
    }

    /// Waits until `url` is there (or, with `present` false, gone), watching its folder.
    func waitFor(_ url: URL, present: Bool = true, timeout: TimeInterval = 20) async -> Bool {
        await watch(url.deletingLastPathComponent(), timeout: timeout) { FileManager.default.fileExists(atPath: url.path) == present }
    }

    private func waitForExit(timeout: TimeInterval) async -> Bool {
        let (stream, events) = AsyncStream<Void>.makeStream(bufferingPolicy: .bufferingNewest(1))
        let deadline = DispatchWorkItem { @Sendable in events.finish() }
        DispatchQueue.global().asyncAfter(deadline: .now() + timeout, execute: deadline)
        ended.onEnd { @Sendable in events.finish() }
        defer {
            deadline.cancel()
            ended.onEnd(nil)
        }
        for await _ in stream {}
        return ended.isDone
    }

    // MARK: - Checking

    /// A failed check: recorded, with the control files and the app's output attached the first time.
    func expect(_ ok: Bool, _ message: @autoclosure () -> String, sourceLocation: SourceLocation = #_sourceLocation) {
        if !ok { fail(message(), sourceLocation: sourceLocation) }
    }

    func expect(_ text: String, contains needle: String, sourceLocation: SourceLocation = #_sourceLocation) {
        expect(text.contains(needle), "expected \"\(needle)\" in:\n\(text)", sourceLocation: sourceLocation)
    }

    func fail(_ message: String, sourceLocation: SourceLocation = #_sourceLocation) {
        failed = true
        scratch.markFailed()
        Issue.record(Comment(rawValue: message), sourceLocation: sourceLocation)
        guard !attached else { return }
        attached = true
        let names = ((try? FileManager.default.contentsOfDirectory(atPath: scratch.control.path)) ?? []).filter { $0.hasPrefix("out") }.sorted()
        for name in names {
            if let text = try? String(contentsOf: scratch.control.appendingPathComponent(name), encoding: .utf8) {
                Attachment.record(text, named: name, sourceLocation: sourceLocation)
            }
        }
        if let log = try? String(contentsOf: scratch.root.appendingPathComponent("app.log"), encoding: .utf8) {
            Attachment.record(log, named: "app.log", sourceLocation: sourceLocation)
        }
    }

    // MARK: - The app's own hook client

    /// `Wormhole hook --agent <agent> <event>` as an agent's hook script runs it, from terminal `terminal`: the exit status.
    @discardableResult
    func hook(agent: String, event: String, terminal: Int) throws -> Int32 {
        try Spawn.run(program.path, ["hook", "--agent", agent, event, "{}"], environment: clientEnvironment(terminal: terminal)).status
    }

    /// What the app's own client is given to reach this instance: its socket, its data folder, and the terminal it runs in (none:
    /// a program outside the app's terminals).
    func clientEnvironment(terminal: Int?) throws -> [String: String] {
        let names = (try? FileManager.default.contentsOfDirectory(atPath: scratch.temp.path)) ?? []
        guard let socket = names.first(where: { $0.hasPrefix("wormhole-") && $0.hasSuffix(".sock") }) else {
            throw ScratchError(description: "the app serves no hook socket in \(scratch.temp.path)")
        }
        var environment = scratch.cleanEnvironment
        environment["WORMHOLE_SOCKET"] = scratch.temp.appendingPathComponent(socket).path
        environment["WORMHOLE_DATA_DIR"] = scratch.data.path
        if let terminal { environment["WORMHOLE_TAB_ID"] = String(terminal) }
        return environment
    }
}

/// Whether the app has ended, told from the process's own queue.
private final class Ending: Sendable {
    private struct State {
        var done = false
        var wake: (@Sendable () -> Void)?
    }
    private let state = Mutex(State())

    var isDone: Bool { state.withLock { $0.done } }

    func reset() { state.withLock { $0 = State() } }

    func finish(_ status: Int32) {
        let wake = state.withLock { state -> (@Sendable () -> Void)? in
            state.done = true
            return state.wake
        }
        wake?()
    }

    /// What to do when the app ends (at once when it has already); nil forgets it.
    func onEnd(_ wake: (@Sendable () -> Void)?) {
        let now = state.withLock { state -> Bool in
            state.wake = wake
            return state.done && wake != nil
        }
        if now { wake?() }
    }
}
