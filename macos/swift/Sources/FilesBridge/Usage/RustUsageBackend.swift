// `UsageBackend` over core's `agents::usage`: the plan limits the agents last reported, kept across restarts. Where
// the reports come from is the host's: the status line command and the hooks reach the app on its socket, and the
// host hands what arrives to `recordStatusLine`, `recordHook` and `recordReport`.

import Foundation
import os
import PanelModel
import WormholeBindings

public final class RustUsageBackend: UsageBackend, @unchecked Sendable {
    /// The core's service, shared with the hook socket that hands it the agents' reports.
    public let service: UsageService
    private let relay: UsageRelay
    /// The profile this window's meters are of (reports are kept per profile); nil reads the profile Wormhole is in.
    private let profile: OSAllocatedUnfairLock<String?>

    init(service: UsageService, relay: UsageRelay, profile: String? = nil) {
        self.service = service
        self.relay = relay
        self.profile = OSAllocatedUnfairLock(initialState: profile)
    }

    /// A backend over the same service for a window of `profile`: the app has one service, each window its own profile's reports.
    public func bound(to profile: String) -> RustUsageBackend { RustUsageBackend(service: service, relay: relay, profile: profile) }

    /// The window moved to `profile`: its meters are that profile's from now on.
    public func follow(profile id: String) {
        profile.withLock { $0 = id }
        relay.wake()
    }

    /// Wormhole went into `profile`: what agents report is added to that profile's reports.
    public func setShown(profile id: String) { _ = service.followProfile(profile: id) }

    public func reports() async -> [UsageReport] {
        let (service, mine) = (service, profile.withLock { $0 })
        return await offMain { (mine.flatMap { service.profileReports(profile: $0) } ?? service.reports()).map(UsageReport.init) }
    }

    public func changes() -> AsyncStream<Void> { relay.stream() }

    public func nextChange(from date: Date) -> Duration? {
        let now = UInt64(max(date.timeIntervalSince1970 * 1000, 0))
        let wait = profile.withLock { $0 }.map { service.profileNextChange(profile: $0, nowMs: now) } ?? service.nextChange(nowMs: now)
        return wait.map { .milliseconds($0) }
    }

    /// Claude Code's status line input (its JSON as the command reads it from standard input).
    @discardableResult
    public func recordStatusLine(_ json: String, now: Date = Date()) -> Bool {
        service.recordStatusLine(input: json, now: Int64(now.timeIntervalSince1970))
    }

    /// A hook of Codex or Gemini, with the agent's own folder (a transcript is read from under it, never from
    /// anywhere else).
    @discardableResult
    public func recordHook(agent: String, event: String, input: String, home: String, now: Date = Date()) -> Bool {
        service.recordHook(agent: agent, event: event, input: input, home: home, now: Int64(now.timeIntervalSince1970))
    }

    /// A report as the hook socket carries it (a `Usage` as JSON).
    @discardableResult
    public func recordReport(json: String) -> Bool { service.recordReport(json: json) }
}

extension UsageReport {
    init(_ record: UsageReportRecord) {
        self.init(
            agent: record.agent, name: record.name, monogram: record.monogram,
            logo: record.logo.map { AgentLogoShape(paths: $0.paths, evenOdd: $0.evenOdd) },
            limits: record.limits.map { PlanLimit(label: $0.label, used: $0.used, resetsAt: $0.resetsAt.map(Int.init)) },
            responseTokens: record.responseTokens.map(Int.init), updated: Int(record.updated))
    }
}

/// The one listener the usage service has, shared by every `changes()` stream.
final class UsageRelay: PanelListener, @unchecked Sendable {
    private let lock = NSLock()
    private var streams: [UUID: AsyncStream<Void>.Continuation] = [:]
    private weak var service: UsageService?

    func attach(_ service: UsageService) { self.service = service }

    func wake() {
        lock.lock()
        let listening = Array(streams.values)
        lock.unlock()
        listening.forEach { $0.yield() }
        // Re-armed shortly after: reports that arrive together are one wake.
        DispatchQueue.global(qos: .utility).asyncAfter(deadline: .now() + .milliseconds(100)) { [weak self] in
            self?.service?.drain()
        }
    }

    func stream() -> AsyncStream<Void> {
        AsyncStream(bufferingPolicy: .bufferingNewest(1)) { continuation in
            let id = UUID()
            lock.lock()
            streams[id] = continuation
            lock.unlock()
            continuation.onTermination = { [weak self] _ in
                self?.lock.lock()
                self?.streams[id] = nil
                self?.lock.unlock()
            }
        }
    }
}
