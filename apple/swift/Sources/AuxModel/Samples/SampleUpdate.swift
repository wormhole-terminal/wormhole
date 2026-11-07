// An updater that finds a release and walks it through download and install on a timer: the window's whole
// flow on sample data (previews, tests, the app until the core's updater is behind the protocol).

import Foundation

@MainActor
public final class SampleUpdateBackend: UpdateBackend {
    public private(set) var phase: UpdatePhase
    public var channel: UpdateChannel = .stable
    public private(set) var lastChecked: Date?
    public var running: [RunningTerminal]
    public private(set) var restarts: [Bool] = []
    private var continuations: [UUID: AsyncStream<UpdatePhase>.Continuation] = [:]

    public static let release = Release(
        version: "1.7.0",
        notes: "## Added\n- Persistent terminals keep running when the app quits.\n- A Preview update channel.\n\n## Fixed\n- The sidebar no longer jumps while a terminal prints.",
        published: Date(timeIntervalSince1970: 1_793_000_000))

    public init(phase: UpdatePhase = .available(SampleUpdateBackend.release), running: [RunningTerminal] = []) {
        (self.phase, self.running) = (phase, running)
    }

    private func set(_ new: UpdatePhase) {
        phase = new
        for continuation in continuations.values { continuation.yield(new) }
    }

    public func phases() -> AsyncStream<UpdatePhase> {
        let id = UUID()
        return AsyncStream { continuation in
            continuation.yield(phase)
            continuations[id] = continuation
            continuation.onTermination = { _ in Task { @MainActor in self.continuations[id] = nil } }
        }
    }

    public func check() async {
        set(.checking)
        try? await Task.sleep(for: .milliseconds(400))
        lastChecked = Date()
        set(.available(Self.release))
    }

    public func install(_ release: Release) async {
        for step in 1...4 {
            set(.downloading(received: Int64(step) * 5_000_000, expected: 20_000_000))
            try? await Task.sleep(for: .milliseconds(250))
        }
        set(.installed(version: release.version))
    }

    public func skip(_ release: Release) { set(.upToDate) }
    public func restart(whenIdle: Bool) { restarts.append(whenIdle) }
    public func runningTerminals() -> [RunningTerminal] { running }
}
