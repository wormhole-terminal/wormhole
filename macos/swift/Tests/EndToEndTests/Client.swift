import Foundation
import Synchronization

/// The app's own command line (`Wormhole <words…>`) asking the instance through its socket, as an agent or a script would, left
/// running while its request waits on the person's answer (the control folder's `agent-ask`). Its output goes to a file of the
/// scratch folder; `finish` waits for its end on the process's own exit event and a deadline, never a pause.
@MainActor
final class Client {
    private let process: Process
    private let log: URL
    private let state: ClientState

    fileprivate init(process: Process, log: URL, state: ClientState) {
        (self.process, self.log, self.state) = (process, log, state)
    }

    var isRunning: Bool { !state.isDone }

    /// What it printed so far.
    var output: String { (try? String(contentsOf: log, encoding: .utf8)) ?? "" }

    /// Its exit status once it has ended, nil when it is still waiting after `timeout` seconds (it is then stopped).
    func finish(timeout: TimeInterval = 20) async -> Int32? {
        let (stream, events) = AsyncStream<Void>.makeStream(bufferingPolicy: .bufferingNewest(1))
        let deadline = DispatchWorkItem { @Sendable in events.finish() }
        DispatchQueue.global().asyncAfter(deadline: .now() + timeout, execute: deadline)
        state.onEnd { @Sendable in events.finish() }
        defer {
            deadline.cancel()
            state.onEnd(nil)
        }
        for await _ in stream {}
        if let status = state.status { return status }
        process.terminate()
        return nil
    }
}

extension Instance {
    /// `Wormhole <words…>` started against this instance, from inside terminal `terminal` (none: from outside the app's terminals).
    func cli(_ words: [String], terminal: Int? = nil) throws -> Client {
        let log = scratch.root.appendingPathComponent("client-\(UUID().uuidString.prefix(6)).txt")
        let state = ClientState()
        let process = try Spawn.start(program.path, words, environment: try clientEnvironment(terminal: terminal), log: log) { state.finish($0) }
        return Client(process: process, log: log, state: state)
    }

    /// `Wormhole <words…>` run to its end: its status and what it printed.
    func cliRun(_ words: [String], terminal: Int? = nil, timeout: TimeInterval = 20) async throws -> (status: Int32?, output: String) {
        let client = try cli(words, terminal: terminal)
        let status = await client.finish(timeout: timeout)
        return (status, client.output)
    }
}

/// Whether the client has ended and with what status, told from the process's own queue.
private final class ClientState: Sendable {
    private struct Held {
        var status: Int32?
        var wake: (@Sendable () -> Void)?
    }
    private let held = Mutex(Held())

    var isDone: Bool { held.withLock { $0.status != nil } }
    var status: Int32? { held.withLock { $0.status } }

    func finish(_ status: Int32) {
        let wake = held.withLock { held -> (@Sendable () -> Void)? in
            held.status = status
            return held.wake
        }
        wake?()
    }

    /// What to do when the client ends (at once when it has already); nil forgets it.
    func onEnd(_ wake: (@Sendable () -> Void)?) {
        let now = held.withLock { held -> Bool in
            held.wake = wake
            return held.status != nil && wake != nil
        }
        if now { wake?() }
    }
}
