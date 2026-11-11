// The app's hook socket and control line, running (`HookServer` of `wormhole-ffi`; `apple/FFI-CONTRACT.md`, "The
// hook socket and the control line"). The agents' hooks and `Wormhole list|send-text|…` reach the workspace
// without Swift; what only the app can do arrives as `HookAction`s.

import Foundation
import WormholeBindings

@MainActor
public final class HookServerHandle {
    private let server: HookServer
    private let wake = HookWake()

    /// What the server asks the app to do, one array per wake. Ends when `stop()` is called.
    public var actions: AsyncStream<[HookAction]> {
        let (stream, continuation) = AsyncStream<[HookAction]>.makeStream(bufferingPolicy: .unbounded)
        let wake = wake, server = server
        let task = Task.detached {
            for await _ in wake.stream {
                // Reading re-arms the wake, so a request that comes during this read wakes again.
                let events = server.drainEvents().map(HookAction.init)
                if !events.isEmpty { continuation.yield(events) }
            }
            continuation.finish()
        }
        continuation.onTermination = { _ in task.cancel() }
        return stream
    }

    init(workspace: Workspace, usage: UsageService, access: AccessDesk, dataDir: URL?) throws {
        let wake = wake
        server = try HookServer.start(
            workspace: workspace, usage: usage, access: access, listener: wake, dataDir: dataDir?.path)
    }

    /// Where the app listens: what `WORMHOLE_SOCKET` holds in this data folder's terminals.
    public var socketPath: String { server.socketPath() }

    /// The question a `.closeAsked` put to the person was answered (yes or no): another may be asked.
    public func closeQuestionOver() { server.closeQuestionOver() }

    /// Take approval `id` back and end the streams it let in (use this, not `AccessDesk.revoke`, so they end).
    @discardableResult
    public func revokeApproval(profile: String, _ id: UInt64) -> Bool { server.revokeApproval(profile: profile, id: id) }

    /// Take stored approval `id` (a database row of `AccessDesk.approvals()`) back and end the streams it let in.
    @discardableResult
    public func revokeStoredApproval(profile: String, _ id: UInt64) -> Bool {
        server.revokeStoredApproval(profile: profile, id: id)
    }

    /// Another profile's desk: what the requests reaching that profile's windows are judged by.
    public func addDesk(_ desk: AccessDesk) { server.addDesk(desk: desk) }

    /// Profile `profile`'s settings changed: the server reads its agent-access mode again.
    public func settingsChanged(profile: String) { server.settingsChanged(profile: profile) }

    /// The status items the Probes' hooks set (`HookAction.probesChanged` says to read them again).
    public func probeStatusItems() -> [ProbeStatusRecord] { server.probeStatusItems() }

    /// Whether the terminal welcome is on (the feature flag `terminal_welcome`).
    public var welcomeAvailable: Bool { server.welcomeAvailable() }

    /// A welcome starts in terminal `tab` (`handover`: `cursor`, `portal` or `logo`): the token for that terminal's
    /// environment alone, nil when none could be made (the terminal is then a plain one). `animates`: the start animation
    /// is about to play around the window. `importHome`: where an import reads the other terminal's files (nil: the Mac's home).
    public func beginWelcome(
        tab: Int64, handover: String, firstRun: Bool, animates: Bool, reduceMotion: Bool, importHome: String?,
        fonts: FontFamilyResolver
    ) -> String? {
        server.beginWelcome(
            tab: tab,
            plan: WelcomePlan(
                handover: handover, firstRun: firstRun, animates: animates, reduceMotion: reduceMotion, importHome: importHome),
            fonts: fonts)
    }

    /// The start animation really started, or is over: the program's wait counts from here.
    public func welcomePlaying(tab: Int64, _ playing: Bool) { server.welcomePlaying(tab: tab, playing: playing) }

    /// What macOS says of banners (`checking`, `not_asked`, `allowed`, `denied`, `outside`).
    public func setNotificationPermission(_ word: String) { server.setNotificationPermission(state: word) }

    /// The welcome's terminal is gone: its token opens nothing. Whether a first run was still running in it.
    @discardableResult
    public func welcomeGone(tab: Int64) -> Bool { server.welcomeGone(tab: tab) }

    /// Stop serving: the socket goes, the lock is given up, the streams end.
    public func stop() {
        server.stop()
        wake.finish()
    }
}
