// `UpdateBackend` over core's updater (`UpdateEngine`): the check, the download with its minisign check, the swap
// of the bundle and the relaunch are Rust's; this follows its state and hands the window the values it draws.
// Quitting the app and watching for idle terminals are the app's, so they come in as `Hooks`.

import AuxModel
import Foundation
import WormholeBindings

@MainActor
public final class FFIUpdateBackend: UpdateBackend {
    /// What the app around the updater provides.
    public struct Hooks {
        /// The terminals a restart would end, for the question before it.
        public var runningTerminals: @MainActor () -> [RunningTerminal]
        /// Ends the app (the relaunch helper opens the new bundle once this process has exited).
        public var terminate: @MainActor () -> Void
        /// Runs `restart` once every terminal has been idle for a while (`update::restart`); the default runs it now.
        public var whenIdle: @MainActor (_ restart: @escaping @MainActor () -> Void) -> Void

        public init(
            runningTerminals: @escaping @MainActor () -> [RunningTerminal] = { [] },
            terminate: @escaping @MainActor () -> Void,
            whenIdle: @escaping @MainActor (_ restart: @escaping @MainActor () -> Void) -> Void = { $0() }
        ) {
            (self.runningTerminals, self.terminate, self.whenIdle) = (runningTerminals, terminate, whenIdle)
        }
    }

    /// Where and what to update: the release build's facts (a dev build has no feed and passes none).
    public struct Setup {
        /// `<base>/<channel>/latest.json` is the feed.
        public var baseURL: String
        /// The minisign public key (`packaging/updater.pub`).
        public var publicKey: String
        public var currentVersion: String
        public var bundleID: String
        public var teamID: String?
        /// The `.app` to replace; `nil` for a build that does not run from one (it can check, not install).
        public var appBundle: URL?
        /// `codesign --verify` on the new bundle: on for a release, off only for fixtures.
        public var verifyCodesign = true
        /// A GitHub releases URL read when the feed does not answer.
        public var fallbackURL: String?
        /// Where the check schedule and the skipped version are kept; `nil` is the data folder's own.
        public var scheduleFile: URL?
        public var channel: UpdateChannel = .stable

        public init(baseURL: String, publicKey: String, currentVersion: String, bundleID: String) {
            (self.baseURL, self.publicKey, self.currentVersion, self.bundleID) = (baseURL, publicKey, currentVersion, bundleID)
        }

        var record: UpdateSetup {
            UpdateSetup(
                baseUrl: baseURL, publicKey: publicKey, currentVersion: currentVersion, bundleId: bundleID, teamId: teamID,
                appBundle: appBundle?.path, verifyCodesign: verifyCodesign, fallbackUrl: fallbackURL,
                scheduleFile: scheduleFile?.path, channel: channel.record)
        }
    }

    public private(set) var phase: UpdatePhase = .idle
    public private(set) var lastChecked: Date?
    public var channel: UpdateChannel {
        get { snapshotChannel }
        set {
            engine.setChannel(channel: newValue.record)
            apply(engine.state())
        }
    }

    private let engine: UpdateEngine
    private let hooks: Hooks
    private var snapshotChannel: UpdateChannel = .stable
    private var watchers: [UUID: AsyncStream<UpdatePhase>.Continuation] = [:]
    private var pump: Task<Void, Never>?
    private var checks: Task<Void, Never>?

    public init(setup: Setup, hooks: Hooks) throws {
        let hub = WakeHub()
        engine = try UpdateEngine(setup: setup.record, listener: hub)
        self.hooks = hooks
        apply(engine.state())
        let wakes = hub.stream()
        pump = Task { [weak self] in
            for await _ in wakes {
                guard let self else { return }
                apply(engine.state())
            }
        }
    }

    deinit {
        pump?.cancel()
        checks?.cancel()
    }

    public func phases() -> AsyncStream<UpdatePhase> {
        let key = UUID()
        return AsyncStream(bufferingPolicy: .bufferingNewest(1)) { continuation in
            continuation.yield(phase)
            watchers[key] = continuation
            continuation.onTermination = { [weak self] _ in Task { @MainActor in self?.watchers[key] = nil } }
        }
    }

    public func check() async {
        engine.check(manual: true)
        apply(engine.state())
        for await next in phases() where next != .checking { return }
    }

    /// A check nobody asked for (the schedule's): it stays quiet about what the person need not hear (a skipped version, a failure).
    public func checkAutomatically() {
        engine.check(manual: false)
        apply(engine.state())
    }

    public func install(_ release: Release) async {
        do { try engine.install(version: release.version) } catch { phase = .failed("\(error)"); publish() }
        apply(engine.state())
    }

    public func skip(_ release: Release) {
        try? engine.skip(version: release.version)
        apply(engine.state())
    }

    public func restart(whenIdle: Bool) {
        let relaunch: @MainActor () -> Void = { [weak self] in
            guard let self else { return }
            do {
                try engine.restart()
                hooks.terminate()
            } catch {
                apply(engine.state())
            }
        }
        if whenIdle { hooks.whenIdle(relaunch) } else { relaunch() }
    }

    public func runningTerminals() -> [RunningTerminal] { hooks.runningTerminals() }

    public func apply(autoUpdate: Bool, anonymousCount: Bool) {
        engine.setAutoUpdate(on: autoUpdate)
        engine.setAnonymousCount(on: anonymousCount)
        guard autoUpdate != (checks != nil) else { return }
        checks?.cancel()
        checks = nil
        guard autoUpdate else { return }
        checks = Task { [weak self] in
            // Automatic checks as the GPUI app's: one at launch when the last is not recent, then each time the six hours run out.
            if self?.engine.dueAtLaunch() == true { self?.engine.check(manual: false) }
            while let seconds = self?.engine.nextCheckInSeconds() {
                // one-shot: the schedule's next check, armed again after each (a timer for six hours, not a poll)
                try? await Task.sleep(for: .seconds(max(seconds, 60)))
                guard !Task.isCancelled else { return }
                self?.engine.check(manual: false)
            }
        }
    }

    private func apply(_ state: UpdateStateRecord) {
        let next = state.phase.value
        snapshotChannel = state.channel.value
        lastChecked = state.lastChecked.map { Date(timeIntervalSince1970: TimeInterval($0)) }
        if next != phase {
            phase = next
            publish()
        }
    }

    private func publish() { for continuation in watchers.values { continuation.yield(phase) } }
}

extension UpdateChannel {
    var record: UpdateChannelRecord { self == .stable ? .stable : .preview }
}

extension UpdateChannelRecord {
    var value: UpdateChannel { self == .stable ? .stable : .preview }
}

extension UpdatePhaseRecord {
    var value: UpdatePhase {
        switch self {
        case .idle: .idle
        case .checking: .checking
        case .upToDate: .upToDate
        case .available(let release):
            .available(
                Release(
                    version: release.version, notes: release.notes,
                    published: release.published.flatMap { try? Date($0, strategy: .iso8601) }, isSecurity: release.isSecurity))
        case .unsupported(let reason): .unsupported(reason)
        case .downloading(let received, let expected): .downloading(received: Int64(received), expected: Int64(expected))
        case .installed(let version): .installed(version: version)
        case .failed(let message): .failed(message)
        }
    }
}
