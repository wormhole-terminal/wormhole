// The update window's state: the backend's phase as it changes, what the buttons ask of it, and how long ago it
// checked. The restart question is built here from the terminals a restart would end (`update::restart`).

import Foundation
import Observation

@MainActor @Observable
public final class UpdateViewModel {
    public private(set) var phase: UpdatePhase
    public private(set) var lastChecked: Date?
    public var channel: UpdateChannel { didSet { backend.channel = channel } }
    @ObservationIgnored public let backend: any UpdateBackend
    @ObservationIgnored private var follow: Task<Void, Never>?

    public init(backend: any UpdateBackend) {
        self.backend = backend
        phase = backend.phase
        lastChecked = backend.lastChecked
        channel = backend.channel
    }

    /// Follows the backend until `stop`: every phase it reports replaces `phase`.
    public func start() {
        follow?.cancel()
        follow = Task { [weak self, backend] in
            for await next in backend.phases() {
                guard let self else { return }
                phase = next
                lastChecked = backend.lastChecked
            }
        }
    }

    public func stop() { follow?.cancel() }

    public func check() { Task { await backend.check() } }
    public func install(_ release: Release) { Task { await backend.install(release) } }
    public func skip(_ release: Release) { backend.skip(release) }

    /// The sheet before restarting, or `nil` when nothing would end and the restart needs no question.
    public var restartQuestion: Question? { RestartQuestion.question(backend.runningTerminals()) }

    public func restart(whenIdle: Bool) { backend.restart(whenIdle: whenIdle) }

    /// "Checked just now." and so on; "Not checked yet." before the first check.
    public func checkedLine(now: Date = Date()) -> String {
        guard let lastChecked else { return AuxText.notCheckedYet }
        let age = now.timeIntervalSince(lastChecked)
        if age < 60 { return AuxText.checkedJustNow }
        return AuxText.checked(ago: RelativeDateTimeFormatter().localizedString(for: lastChecked, relativeTo: now))
    }

    /// Bytes as the download line says them ("5.0 MB of 20.0 MB"); `nil` while the size is unknown.
    public static func progressText(received: Int64, expected: Int64) -> String? {
        guard expected > 0 else { return nil }
        let style = ByteCountFormatter()
        return AuxText.downloadProgress(style.string(fromByteCount: received), style.string(fromByteCount: expected))
    }

    /// The fraction downloaded, for a determinate bar; `nil` while the size is unknown.
    public static func fraction(received: Int64, expected: Int64) -> Double? {
        expected > 0 ? min(max(Double(received) / Double(expected), 0), 1) : nil
    }
}
